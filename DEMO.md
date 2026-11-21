# Demo: "Bet Between Friends" on Solana (~3 minutes)

## One-liner
Two friends bet SOL, and a program holds the money until a judge they both trust picks the winner.
Nobody can run off with the pot, not even the judge.

## Script

**1. The problem (20 sec)**
"When friends bet, someone has to hold the money, and you have to trust them.
I wrote a Solana program that holds it instead. The rules are code, and nobody can change them."

**2. The flow (60 sec)**: show `lib.rs` in Playground and scroll to each function
- `create_bet`: player A sets the terms (description, stake, who B is, who judges) and deposits.
  The SOL goes into a **bet account owned by the program**. It's a PDA (an address the program derives itself),
  so there's no private key for it and nobody can sign it away.
- `accept_bet`: player B deposits the same stake. Now the pot is locked.
- `settle`: **only the judge** can call it, and the winner gets the whole pot.
  The judge can only pick A or B, never send the money to themselves.
- `cancel_bet`: A can back out and get the stake back, **but only before B joins**.

**3. Security, where the program refuses (40 sec)**: scroll to `BetError`
- Wrong person accepting: `NotPlayerB`
- Anyone but the judge settling: `NotJudge` (enforced by Anchor's `has_one = judge` + `Signer`)
- Winner who isn't A or B: `InvalidWinner`
- Settling before both paid: `NotAccepted`
- Zero stake, the same wallet as both players, or a judge who is also a player: rejected at creation
- Checked math on the payout (no overflow)

**4. Proof it works (30 sec)**
- Click **Build**: "Build successful."
- Open `anchor.test.ts`: the tests run the whole story. A creates, B accepts,
  a non-judge tries to settle and gets rejected, the judge picks B, and B's balance goes up by exactly
  the pot. A second bet gets cancelled and refunded.
- Not deployed yet because the devnet faucet blocked new GitHub accounts.
  With 2-3 devnet SOL it deploys and the tests run in about a minute.

**5. What's next (20 sec)**
- A **deadline**: if the judge never decides, both players get refunds after X days
  (right now the money would be stuck)
- Bets in USDC or other tokens instead of SOL
- A simple web page with wallet connect so friends can bet from their phones

## Likely questions
- **Where is the money actually held?** In the bet account itself. The program owns it, so only this program's rules can move lamports out.
- **What stops the judge from cheating?** They can only choose A or B. They can be biased, but they can't steal.
- **Why does the rent go back to A?** A paid to create the account. Closing it refunds that deposit.
- **Why Anchor?** Account checks (`has_one`, `seeds`, `Signer`) are declared instead of hand-written, which removes a whole class of bugs.
- **What happens if the judge disappears?** Today the money is stuck. That's the first thing I'd add (deadline + refund).
