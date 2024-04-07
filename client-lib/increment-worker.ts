import {
  Connection,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  clusterApiUrl,
  PublicKey,
  Keypair,
  SystemProgram
} from "@solana/web3.js";
import "dotenv/config";
import { getKeypairFromEnvironment } from "@solana-developers/helpers";
import * as borsh from 'borsh';

const echoSchema = { struct: {
  variant: 'u8', id: 'string', number: 'u64'
}};

const programId = new PublicKey("D1JKf9t3tEBzP7jES8bUzCQdLSYSqfcJ2S558AbQruJm");

const senderKeypair = getKeypairFromEnvironment(process.argv[2]);
const workerKeypair = getKeypairFromEnvironment(process.argv[3]);
const id = process.argv[4];

const [pda_contract] = PublicKey.findProgramAddressSync(
  [
    senderKeypair.publicKey.toBuffer(),
    workerKeypair.publicKey.toBuffer(),
    Buffer.from(id)
  ],
  programId 
);

const instructionData = borsh
  .serialize(echoSchema, { variant: 4, id: id, number: 0 });

console.log(`Sender: `, senderKeypair.publicKey.toBase58());

const connection = new Connection(clusterApiUrl("devnet"));
const transaction = new Transaction();

const from = senderKeypair.publicKey;

const instruction = new TransactionInstruction({
  keys: [
    {
      pubkey: senderKeypair.publicKey,
      isSigner: true,
      isWritable: true
    },
    {
      pubkey: workerKeypair.publicKey,
      isSigner: false,
      isWritable: true
    },
    {
      pubkey: pda_contract,
      isSigner: false,
      isWritable: true
    },
    {
      pubkey: SystemProgram.programId,
      isSigner: false,
      isWritable: false
    }
  ],
  programId: programId,
  data: Buffer.from(instructionData)
});

transaction.add(instruction);

const signature = await sendAndConfirmTransaction(connection, transaction, [senderKeypair]);

console.log(`Transaction signature: ` + signature);
