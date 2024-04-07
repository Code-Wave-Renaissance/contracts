use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh1::try_from_slice_unchecked,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

#[derive(BorshDeserialize)]
pub struct InstructionPayload {
    pub contract_id: String,
    pub total_quantity: u64,
}

pub enum Instruction {
    CreateContract { contract_id: String, total_quantity: u64 },
    IncrementStep { contract_id: String },
}

impl Instruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        let payload = InstructionPayload::try_from_slice(rest).unwrap();

        Ok(match variant {
            0 => Self::CreateContract {
                contract_id: payload.contract_id,
                total_quantity: payload.total_quantity,
            },
            1 => Self::IncrementStep {
                contract_id: payload.contract_id
            },
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ContractData {
    pub contract_id: String,
    pub owner: Pubkey,
    pub worker: Pubkey,
    pub total_quantity: u64,
    pub actual_step: u64,
}

impl ContractData {
    pub fn get_account_size(contract_id: String) -> usize {
        return 1
            + 4
            + contract_id.len()
            + (2 * std::mem::size_of::<Pubkey>())
            + (2 * std::mem::size_of::<u64>());
    }
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Instruction::unpack(instruction_data)?;

    match instruction {
        Instruction::CreateContract { contract_id, total_quantity } => {
            create_contract(program_id, accounts, contract_id, total_quantity)?;
        }
        Instruction::IncrementStep { contract_id } => {
            increment_step(program_id, accounts, contract_id)?;
        }
    }

    Ok(())
}

fn create_contract(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    contract_id: String,
    total_quantity: u64,
) -> ProgramResult {

    let account_info_iter = &mut accounts.iter();

    let sender = next_account_info(account_info_iter)?;
    let worker = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if total_quantity > sender.lamports() {
        return Err(ProgramError::InsufficientFunds);
    }

    let (pda_account_key, bump_seed) = Pubkey::find_program_address(
        &[
            sender.key.as_ref(),
            worker.key.as_ref(),
            contract_id.as_bytes().as_ref(),
        ],
        program_id,
    );

    if *pda_account.key != pda_account_key {
        return Err(ProgramError::InvalidAccountData);
    }

    let account_len = ContractData::get_account_size(contract_id.clone());
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    invoke_signed(
        &system_instruction::create_account(
            sender.key,
            pda_account.key,
            rent_lamports,
            account_len.try_into().unwrap(),
            program_id,
        ),
        &[sender.clone(), pda_account.clone(), system_program.clone()],
        &[&[
            sender.key.as_ref(),
            worker.key.as_ref(),
            contract_id.as_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;

    invoke(
        &system_instruction::transfer(sender.key, pda_account.key, total_quantity),
        &[sender.clone(), pda_account.clone(), system_program.clone()],
    )?;

    msg!("PDA create - {}", pda_account_key);

    let mut account_data =
        try_from_slice_unchecked::<ContractData>(&pda_account.data.borrow()).unwrap();

    account_data.contract_id = contract_id;
    account_data.owner = sender.key.clone();
    account_data.worker = worker.key.clone();
    account_data.total_quantity = total_quantity;
    account_data.actual_step = 0;

    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;

    Ok(())
}

fn increment_step(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    contract_id: String
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let sender = next_account_info(account_info_iter)?;
    let worker = next_account_info(account_info_iter)?;
    let pda_contract = next_account_info(account_info_iter)?;

    let (pda_contract_key, _bump_seed_contract) = Pubkey::find_program_address(
        &[
            sender.key.as_ref(),
            worker.key.as_ref(),
            contract_id.as_bytes().as_ref(),
        ],
        program_id,
    );

    if *pda_contract.key != pda_contract_key {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut account_data =
        try_from_slice_unchecked::<ContractData>(&pda_contract.data.borrow())
        .unwrap();

    let transfer_amount = match account_data.actual_step {
        0 | 1 => Ok(account_data.total_quantity / 3),
        2 => {
            let quantity_per_three = account_data.total_quantity / 3;
            Ok(account_data.total_quantity - quantity_per_three - quantity_per_three)
        }
        _ => {
            Err(ProgramError::InsufficientFunds)
        }
    }?;

    let pda_initial_amount = pda_contract.lamports();
    let worker_initial_amount = worker.lamports();

    **worker.lamports.borrow_mut() = worker_initial_amount + transfer_amount;
    **pda_contract.lamports.borrow_mut() = pda_initial_amount - transfer_amount;

    msg!(
        "{} lamports transferred from contract to {}",
        transfer_amount,
        worker.key
    );

    account_data.actual_step = account_data.actual_step + 1;

    account_data.serialize(&mut &mut pda_contract.data.borrow_mut()[..])?;

    Ok(())
}
