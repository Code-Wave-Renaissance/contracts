import {
    Connection,
    TransactionInstruction,
    PublicKey,
    SystemProgram
  } from "@solana/web3.js";
  import * as borsh from 'borsh';

  export type ContractData = {
    contractId: String
    owner: PublicKey
    worker: PublicKey
    totalQuantity: Number
    actualStep: Number
  }

  const contractDataSchema = { struct: {
    contract_id: 'string',
    owner: { array: { type: 'u8', len: 32 } },
    worker: { array: { type: 'u8', len: 32 } },
    total_quantity: 'u64',
    actual_step: 'u64',
  }};

  const instructionSchema = { struct: {
    variant: 'u8', contract_id: 'string', total_quantity: 'u64'
  }};
  
  const programId = new PublicKey("D1JKf9t3tEBzP7jES8bUzCQdLSYSqfcJ2S558AbQruJm");

  export function createContract(
      owner: PublicKey,
      worker: PublicKey,
      contractId: String,
      totalQuantity: Number
  ): TransactionInstruction {

      const pda = getPda(owner, worker, contractId);
      const instructionData = getIntructionData(0, contractId, totalQuantity);
  
      return new TransactionInstruction({
          keys: [
            {
              pubkey: owner,
              isSigner: true,
              isWritable: true
            },
            {
              pubkey: worker,
              isSigner:false,
              isWritable: false
            },
            {
              pubkey: pda,
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
  }

  export function incrementStep(
    owner: PublicKey,
    worker: PublicKey,
    contractId: String
  ): TransactionInstruction {
  
      const pda = getPda(owner, worker, contractId);
      const instructionData = getIntructionData(1, contractId, 0);
  
      return new TransactionInstruction({
          keys: [
            {
              pubkey: owner,
              isSigner: true,
              isWritable: true
            },
            {
              pubkey: worker,
              isSigner:false,
              isWritable: true
            },
            {
              pubkey: pda,
              isSigner: false,
              isWritable: true
            }
          ],
          programId: programId,
          data: Buffer.from(instructionData)
        });
  }

  export function getContractData(
    connection: Connection,
    owner: PublicKey,
    worker: PublicKey,
    contractId: String
  ): Promise<ContractData> {
    
    const pda = getPda(owner, worker, contractId);
    return connection.getAccountInfo(pda).then(account => {
        if (account != null)
        {
            const data = borsh.deserialize(contractDataSchema, account.data);
            
            const owner: Uint8Array = data?.valueOf()["owner"];
            const worker: Uint8Array = data?.valueOf()["worker"];

            return {
                contractId: data?.valueOf()["contract_id"],
                owner: new PublicKey(owner),
                worker: new PublicKey(worker),
                totalQuantity: data?.valueOf()["total_quantity"],
                actualStep: data?.valueOf()["actual_step"],
            };
        }
        
        throw new Error("Contract not found");
    });
  }

  function getPda(
    owner: PublicKey,
    worker: PublicKey,
    contractId: String): PublicKey
  {
    const [pda] = PublicKey.findProgramAddressSync(
        [
          owner.toBuffer(),
          worker.toBuffer(),
          Buffer.from(contractId)
        ],
        programId 
    );

    return pda;
  }

  function getIntructionData(variant: Number, contractId: String, totalQuantity: Number)
  {
    return borsh
        .serialize(
            instructionSchema,
            { variant: variant, contract_id: contractId, total_quantity: totalQuantity }
        );
  }