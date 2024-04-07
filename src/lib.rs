use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh0_10::try_from_slice_unchecked,
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
use std::convert::TryInto;

#[derive(BorshDeserialize)]
pub struct HelloInstructionPayload {
    pub id: String,
    pub number: u64,
}

pub enum HelloInstruction {
    Echo { value: String },
    Square { number: u64 },
    CreateContract { id: String, totalQuantity: u64 },
    ShowContract { id: String },
    Increment { id: String, score: u64 },
}

impl HelloInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        let payload = HelloInstructionPayload::try_from_slice(rest).unwrap();

        Ok(match variant {
            0 => Self::Echo { value: payload.id },
            1 => Self::Square {
                number: payload.number,
            },
            2 => Self::CreateContract {
                id: payload.id,
                totalQuantity: payload.number,
            },
            3 => Self::ShowContract { id: payload.id },
            4 => Self::Increment {
                id: payload.id,
                score: payload.number,
            },
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct OwnerData {
    pub id: String,
    pub Owner: Pubkey,
    pub Worker: Pubkey,
    pub TotalQuantity: u64,
    pub ActualStep: u64,
}

impl OwnerData {
    pub fn get_account_size(id: String) -> usize {
        return 1
            + 4
            + id.len()
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
    let instruction = HelloInstruction::unpack(instruction_data)?;

    match instruction {
        HelloInstruction::Echo { value } => {
            msg!(&value);
        }
        HelloInstruction::Square { number } => {
            let x = number * number;
            msg!("{}", x);
        }
        HelloInstruction::CreateContract { id, totalQuantity } => {
            create_contract(program_id, accounts, id, totalQuantity)?;
        }
        HelloInstruction::ShowContract { id } => {
            show_contract(program_id, accounts, id)?;
        }
        HelloInstruction::Increment { id, score } => {
            increment_worker(program_id, accounts, id, score)?;
        }
    }

    Ok(())
}

fn create_contract(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    id: String,
    totalQuantity: u64,
) -> ProgramResult {
    msg!("Received id: {}", id);
    msg!("Received quantity: {}", totalQuantity);

    let account_info_iter = &mut accounts.iter();

    let sender = next_account_info(account_info_iter)?;
    let worker = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    msg!("Owner balance: {}", sender.lamports());
    if totalQuantity > sender.lamports() {
        return Err(ProgramError::InsufficientFunds);
    }

    let (pda_account_key, bump_seed) = Pubkey::find_program_address(
        &[
            sender.key.as_ref(),
            worker.key.as_ref(),
            id.as_bytes().as_ref(),
        ],
        program_id,
    );

    if *pda_account.key != pda_account_key {
        return Err(ProgramError::InvalidAccountData);
    }

    let account_len = OwnerData::get_account_size(id.clone());
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    msg!("Account Len: {}", account_len);
    msg!("Rent: {}", rent_lamports);
    msg!("Creating PDA");

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
            id.as_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;

    invoke(
        &system_instruction::transfer(sender.key, pda_account.key, totalQuantity),
        &[sender.clone(), pda_account.clone(), system_program.clone()],
    )?;

    msg!("PDA create - {}", pda_account_key);

    let mut account_data =
        try_from_slice_unchecked::<OwnerData>(&pda_account.data.borrow()).unwrap();

    account_data.id = id;
    account_data.Owner = sender.key.clone();
    account_data.Worker = worker.key.clone();
    account_data.TotalQuantity = totalQuantity;
    account_data.ActualStep = 0;

    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;

    Ok(())
}

fn show_contract(program_id: &Pubkey, accounts: &[AccountInfo], id: String) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let sender = next_account_info(account_info_iter)?;
    let worker = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;

    let (pda_account_key, bump_seed) = Pubkey::find_program_address(
        &[
            sender.key.as_ref(),
            worker.key.as_ref(),
            id.as_bytes().as_ref(),
        ],
        program_id,
    );

    if *pda_account.key != pda_account_key {
        return Err(ProgramError::InvalidAccountData);
    }

    let account_data = try_from_slice_unchecked::<OwnerData>(&pda_account.data.borrow()).unwrap();

    msg!("Contract id: {}", account_data.id);
    msg!("Owner pubkey: {}", account_data.Owner);
    msg!("Worker pubkey: {}", account_data.Worker);
    msg!("Total quantity: {}", account_data.TotalQuantity);
    msg!("Actual Step: {}", account_data.ActualStep);

    Ok(())
}

fn increment_worker(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    id: String,
    score: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let sender = next_account_info(account_info_iter)?;
    let worker = next_account_info(account_info_iter)?;
    let pda_contract = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let (pda_contract_key, bump_seed_contract) = Pubkey::find_program_address(
        &[
            sender.key.as_ref(),
            worker.key.as_ref(),
            id.as_bytes().as_ref(),
        ],
        program_id,
    );

    if *pda_contract.key != pda_contract_key {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut account_data =
        try_from_slice_unchecked::<OwnerData>(&pda_contract.data.borrow()).unwrap();

    let toTransfer: u64 = match account_data.ActualStep {
        0 => account_data.TotalQuantity / 3,
        1 => account_data.TotalQuantity / 3,
        2 => {
            let quantityPerThree = account_data.TotalQuantity / 3;
            account_data.TotalQuantity - quantityPerThree - quantityPerThree
        }
        _ => {
            msg!("Actual Step Invalid");
            0
        }
    };

    let pda_initial_amount = pda_contract.lamports();
    let worker_initial_amount = worker.lamports();

    **worker.lamports.borrow_mut() = worker_initial_amount + toTransfer;
    **pda_contract.lamports.borrow_mut() = pda_initial_amount - toTransfer;

    msg!(
        "{} lamports transferred from contract to {}",
        toTransfer,
        worker.key
    );

    account_data.ActualStep = account_data.ActualStep + 1;

    account_data.serialize(&mut &mut pda_contract.data.borrow_mut()[..])?;

    Ok(())
}
