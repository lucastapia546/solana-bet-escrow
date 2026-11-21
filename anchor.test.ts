// Solana Playground test file. `pg`, `web3`, `BN` and `assert` are globals there.
// Your Playground wallet is player A. Player B and the judge are fresh keypairs.

describe("bet", () => {
  const STAKE = new BN(0.05 * web3.LAMPORTS_PER_SOL);
  const playerA = pg.wallet.publicKey;
  const playerB = web3.Keypair.generate();
  const judge = web3.Keypair.generate();

  const betPda = (id: BN) =>
    web3.PublicKey.findProgramAddressSync(
      [Buffer.from("bet"), playerA.toBuffer(), id.toArrayLike(Buffer, "le", 8)],
      pg.PROGRAM_ID
    )[0];

  const balance = (pk: web3.PublicKey) => pg.connection.getBalance(pk, "confirmed");
  const link = (tx: string) => `https://explorer.solana.com/tx/${tx}?cluster=devnet`;

  const betId = new BN(Date.now());
  const bet = betPda(betId);

  before(async () => {
    // Give player B enough SOL for their stake (fees are paid by your wallet).
    const tx = new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey: playerA,
        toPubkey: playerB.publicKey,
        lamports: STAKE.toNumber() + 0.01 * web3.LAMPORTS_PER_SOL,
      })
    );
    await web3.sendAndConfirmTransaction(pg.connection, tx, [pg.wallet.keypair]);
  });

  it("player A creates the bet and deposits", async () => {
    const tx = await pg.program.methods
      .createBet(betId, "Will it rain tomorrow?", STAKE, playerB.publicKey, judge.publicKey)
      .accounts({ bet, playerA, systemProgram: web3.SystemProgram.programId })
      .rpc();
    console.log("create:", link(tx));

    const account = await pg.program.account.bet.fetch(bet);
    assert.ok(!account.accepted);
    assert.equal(account.amount.toString(), STAKE.toString());
  });

  it("player B accepts and deposits", async () => {
    const tx = await pg.program.methods
      .acceptBet()
      .accounts({ bet, playerB: playerB.publicKey, systemProgram: web3.SystemProgram.programId })
      .signers([playerB])
      .rpc();
    console.log("accept:", link(tx));

    const account = await pg.program.account.bet.fetch(bet);
    assert.ok(account.accepted);
  });

  it("someone who isn't the judge cannot settle", async () => {
    let failed = false;
    try {
      await pg.program.methods
        .settle(playerA)
        .accounts({ bet, judge: playerA, playerA, playerB: playerB.publicKey })
        .rpc();
    } catch (e) {
      failed = true;
      console.log("rejected as expected:", e.error?.errorMessage ?? e.message);
    }
    assert.ok(failed, "non-judge settle should fail");
  });

  it("judge picks player B, who gets the whole pot", async () => {
    const before = await balance(playerB.publicKey);

    const tx = await pg.program.methods
      .settle(playerB.publicKey)
      .accounts({ bet, judge: judge.publicKey, playerA, playerB: playerB.publicKey })
      .signers([judge])
      .rpc();
    console.log("settle:", link(tx));

    const after = await balance(playerB.publicKey);
    assert.equal(after - before, STAKE.toNumber() * 2);
    assert.equal(await pg.connection.getAccountInfo(bet), null, "bet account closed");
    console.log("player B won", (after - before) / web3.LAMPORTS_PER_SOL, "SOL");
  });

  it("player A can cancel a bet nobody accepted and get the stake back", async () => {
    const id = new BN(Date.now() + 1);
    const pda = betPda(id);
    await pg.program.methods
      .createBet(id, "Unaccepted bet", STAKE, playerB.publicKey, judge.publicKey)
      .accounts({ bet: pda, playerA, systemProgram: web3.SystemProgram.programId })
      .rpc();

    const tx = await pg.program.methods
      .cancelBet()
      .accounts({ bet: pda, playerA })
      .rpc();
    console.log("cancel:", link(tx));
    assert.equal(await pg.connection.getAccountInfo(pda), null, "bet account closed");
  });
});
