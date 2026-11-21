# Bet Between Friends: a Solana escrow program

**Live UI:** https://lucastapia546.github.io/solana-bet-escrow/ (Demo mode works with no wallet; Devnet mode talks to the deployed program via Phantom)

Two friends bet SOL, and a Solana program holds the money until a judge they both trust picks the winner.
Nobody can run off with the pot, not even the judge.

Built with **Anchor** in **Solana Playground** during a 30-minute workshop.

## How it works

| Instruction | Who can call it | What happens |
|---|---|---|
| `create_bet` | Player A | Sets the terms (description, stake, player B, judge) and deposits A's stake into a program-owned bet account (PDA) |
| `accept_bet` | Player B only | Deposits the matching stake; the pot is now locked |
| `settle` | Judge only | Sends the whole pot to the winner (must be A or B), then closes the bet account |
| `cancel_bet` | Player A only | Refunds A's stake, but only before B has accepted |

The bet account is a **PDA** derived from `("bet", player_a, bet_id)`. It has no private key, so only this program's rules can move the funds.

## Safety checks
- Only player B can accept (`has_one = player_b`), and only the judge can settle (`has_one = judge` + `Signer`)
- The winner must be player A or player B, so the judge can't pay themselves
- The bet can't be settled before both players have deposited, and can't be cancelled after B joins
- Stake must be > 0; the two players must be different wallets; the judge can't be a player
- Checked math on the payout

## Files
- [`lib.rs`](lib.rs): the Anchor program
- [`anchor.test.ts`](anchor.test.ts): Solana Playground tests covering create, accept, a rejected non-judge settle, judge pays B (balance check), and cancel with refund
- [`DEMO.md`](DEMO.md): demo script

## Run it
1. Open [Solana Playground](https://beta.solpg.io) and create an Anchor project
2. Paste `lib.rs` into `src/lib.rs` and the test file into `tests/anchor.test.ts`
3. Fund the Playground wallet with ~3 devnet SOL, then **Build**, **Deploy**, **Test**

**Status:** builds successfully. Not yet deployed to devnet (the devnet faucet was unavailable during the workshop).

## Next steps
- A deadline with automatic refunds if the judge never decides
- SPL token / USDC bets
- A web frontend with wallet connect
