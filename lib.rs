use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("11111111111111111111111111111111");

pub const MAX_DESCRIPTION_LEN: usize = 100;

#[program]
pub mod bet {
    use super::*;

    /// Player A creates the bet and deposits their stake into the bet account.
    pub fn create_bet(
        ctx: Context<CreateBet>,
        bet_id: u64,
        description: String,
        amount: u64,
        player_b: Pubkey,
        judge: Pubkey,
    ) -> Result<()> {
        let player_a = ctx.accounts.player_a.key();
        require!(amount > 0, BetError::ZeroAmount);
        require!(
            !description.is_empty() && description.len() <= MAX_DESCRIPTION_LEN,
            BetError::DescriptionLength
        );
        require!(player_b != player_a, BetError::SamePlayers);
        require!(judge != player_a && judge != player_b, BetError::JudgeIsPlayer);

        // Move player A's stake into the bet account (held by the program).
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.player_a.to_account_info(),
                    to: ctx.accounts.bet.to_account_info(),
                },
            ),
            amount,
        )?;

        let bet = &mut ctx.accounts.bet;
        bet.player_a = player_a;
        bet.player_b = player_b;
        bet.judge = judge;
        bet.bet_id = bet_id;
        bet.amount = amount;
        bet.accepted = false;
        bet.description = description;
        bet.bump = ctx.bumps.bet;

        emit!(BetCreated {
            bet: bet.key(),
            player_a,
            player_b,
            judge,
            amount,
        });
        Ok(())
    }

    /// Player B accepts by depositing the same stake. Now the pot is locked.
    pub fn accept_bet(ctx: Context<AcceptBet>) -> Result<()> {
        require!(!ctx.accounts.bet.accepted, BetError::AlreadyAccepted);
        let amount = ctx.accounts.bet.amount;

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.player_b.to_account_info(),
                    to: ctx.accounts.bet.to_account_info(),
                },
            ),
            amount,
        )?;

        ctx.accounts.bet.accepted = true;
        emit!(BetAccepted {
            bet: ctx.accounts.bet.key(),
        });
        Ok(())
    }

    /// Only the judge can settle. The winner gets the whole pot; the bet
    /// account is then closed and its rent goes back to player A.
    pub fn settle(ctx: Context<Settle>, winner: Pubkey) -> Result<()> {
        let bet = &ctx.accounts.bet;
        require!(bet.accepted, BetError::NotAccepted);
        require!(
            winner == bet.player_a || winner == bet.player_b,
            BetError::InvalidWinner
        );
        let pot = bet.amount.checked_mul(2).ok_or(BetError::Overflow)?;

        let bet_info = ctx.accounts.bet.to_account_info();
        let winner_info = if winner == bet.player_a {
            ctx.accounts.player_a.to_account_info()
        } else {
            ctx.accounts.player_b.to_account_info()
        };

        // The program owns the bet account, so it can move lamports out directly.
        let new_bet_balance = bet_info
            .lamports()
            .checked_sub(pot)
            .ok_or(BetError::Overflow)?;
        let new_winner_balance = winner_info
            .lamports()
            .checked_add(pot)
            .ok_or(BetError::Overflow)?;
        **bet_info.try_borrow_mut_lamports()? = new_bet_balance;
        **winner_info.try_borrow_mut_lamports()? = new_winner_balance;

        emit!(BetSettled {
            bet: ctx.accounts.bet.key(),
            winner,
            pot,
        });
        Ok(())
    }

    /// Player A can cancel and get their stake back, but only before B accepts.
    pub fn cancel_bet(ctx: Context<CancelBet>) -> Result<()> {
        require!(!ctx.accounts.bet.accepted, BetError::AlreadyAccepted);
        emit!(BetCancelled {
            bet: ctx.accounts.bet.key(),
        });
        // `close = player_a` returns the stake + rent to player A.
        Ok(())
    }
}

// ---------- Accounts ----------

#[derive(Accounts)]
#[instruction(bet_id: u64)]
pub struct CreateBet<'info> {
    #[account(
        init,
        payer = player_a,
        space = 8 + Bet::INIT_SPACE,
        seeds = [b"bet", player_a.key().as_ref(), &bet_id.to_le_bytes()],
        bump
    )]
    pub bet: Account<'info, Bet>,
    #[account(mut)]
    pub player_a: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AcceptBet<'info> {
    #[account(
        mut,
        has_one = player_b @ BetError::NotPlayerB,
        seeds = [b"bet", bet.player_a.as_ref(), &bet.bet_id.to_le_bytes()],
        bump = bet.bump
    )]
    pub bet: Account<'info, Bet>,
    #[account(mut)]
    pub player_b: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Settle<'info> {
    #[account(
        mut,
        has_one = judge @ BetError::NotJudge,
        has_one = player_a,
        has_one = player_b,
        seeds = [b"bet", bet.player_a.as_ref(), &bet.bet_id.to_le_bytes()],
        bump = bet.bump,
        close = player_a
    )]
    pub bet: Account<'info, Bet>,
    pub judge: Signer<'info>,
    #[account(mut)]
    pub player_a: SystemAccount<'info>,
    #[account(mut)]
    pub player_b: SystemAccount<'info>,
}

#[derive(Accounts)]
pub struct CancelBet<'info> {
    #[account(
        mut,
        has_one = player_a,
        seeds = [b"bet", player_a.key().as_ref(), &bet.bet_id.to_le_bytes()],
        bump = bet.bump,
        close = player_a
    )]
    pub bet: Account<'info, Bet>,
    #[account(mut)]
    pub player_a: Signer<'info>,
}

// ---------- State ----------

#[account]
#[derive(InitSpace)]
pub struct Bet {
    pub player_a: Pubkey,
    pub player_b: Pubkey,
    pub judge: Pubkey,
    pub bet_id: u64,
    pub amount: u64,
    pub accepted: bool,
    #[max_len(100)]
    pub description: String,
    pub bump: u8,
}

// ---------- Events ----------

#[event]
pub struct BetCreated {
    pub bet: Pubkey,
    pub player_a: Pubkey,
    pub player_b: Pubkey,
    pub judge: Pubkey,
    pub amount: u64,
}

#[event]
pub struct BetAccepted {
    pub bet: Pubkey,
}

#[event]
pub struct BetSettled {
    pub bet: Pubkey,
    pub winner: Pubkey,
    pub pot: u64,
}

#[event]
pub struct BetCancelled {
    pub bet: Pubkey,
}

// ---------- Errors ----------

#[error_code]
pub enum BetError {
    #[msg("Stake must be greater than zero")]
    ZeroAmount,
    #[msg("Description must be 1-100 bytes")]
    DescriptionLength,
    #[msg("Player B must be a different wallet")]
    SamePlayers,
    #[msg("The judge cannot be one of the players")]
    JudgeIsPlayer,
    #[msg("Only player B can accept this bet")]
    NotPlayerB,
    #[msg("This bet was already accepted")]
    AlreadyAccepted,
    #[msg("Both players must deposit before settling")]
    NotAccepted,
    #[msg("Only the judge can settle this bet")]
    NotJudge,
    #[msg("Winner must be player A or player B")]
    InvalidWinner,
    #[msg("Math overflow")]
    Overflow,
}
