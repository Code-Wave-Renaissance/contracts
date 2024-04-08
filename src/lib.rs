use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh1::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    entrypoint,
    msg,
    program::{invoke, invoke_signed},
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

        match variant {
            0 => Ok(
                Self::CreateContract {
                    contract_id: payload.contract_id,
                    total_quantity: payload.total_quantity,
                }
            ),
            1 => Ok(
                Self::IncrementStep { contract_id: payload.contract_id }
            ),
            _ => return Err(ProgramError::InvalidInstructionData),
        }
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
    pub fn get_account_size_and_rent(contract_id: String) -> Result<(usize, u64), ProgramError> {
        let account_len =
            1
            + 4
            + contract_id.len()
            + (2 * std::mem::size_of::<Pubkey>())
            + (2 * std::mem::size_of::<u64>());

        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(account_len);

        Ok((account_len, rent_lamports))
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
            create_contract_handler(program_id, accounts, contract_id, total_quantity)
        }
        Instruction::IncrementStep { contract_id } => {
            increment_step_handler(program_id, accounts, contract_id)
        }
    }
}

fn create_contract_handler(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    contract_id: String,
    total_quantity: u64,
) -> ProgramResult {

    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let worker = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    if total_quantity > owner.lamports() {
        return Err(ProgramError::InsufficientFunds);
    }

    let (pda_key, bump_seed) = Pubkey::find_program_address(
        &[
            owner.key.as_ref(),
            worker.key.as_ref(),
            contract_id.as_bytes().as_ref(),
        ],
        program_id,
    );

    validate_accounts_on_creation(owner, pda, &pda_key)?;

    create_contract(
        program_id,
        owner,
        worker,
        pda,
        system_program,
        bump_seed,
        contract_id,
        total_quantity
    )
}

fn create_contract<'a>(
    program_id: &Pubkey,
    owner: &AccountInfo<'a>,
    worker: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    bump_seed: u8,
    contract_id: String,
    total_quantity: u64
) -> ProgramResult {

    let (account_len, rent_lamports) =
        ContractData::get_account_size_and_rent(contract_id.clone())?;

    invoke_signed(
        &system_instruction::create_account(
            owner.key,
            pda.key,
            rent_lamports,
            account_len.try_into().unwrap(),
            program_id,
        ),
        &[owner.clone(), pda.clone(), system_program.clone()],
        &[&[
            owner.key.as_ref(),
            worker.key.as_ref(),
            contract_id.as_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;

    invoke(
        &system_instruction::transfer(owner.key, pda.key, total_quantity),
        &[owner.clone(), pda.clone(), system_program.clone()],
    )?;

    let mut contract_data =
        try_from_slice_unchecked::<ContractData>(&pda.data.borrow())?;

    contract_data.contract_id = contract_id;
    contract_data.owner = owner.key.clone();
    contract_data.worker = worker.key.clone();
    contract_data.total_quantity = total_quantity;
    contract_data.actual_step = 0;

    contract_data.serialize(&mut &mut pda.data.borrow_mut()[..])?;

    msg!("Contract created - {}", pda.key);

    Ok(())
}

fn increment_step_handler(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    contract_id: String
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let owner = next_account_info(account_info_iter)?;
    let worker = next_account_info(account_info_iter)?;
    let pda = next_account_info(account_info_iter)?;

    let (pda_key, _bump_seed) = Pubkey::find_program_address(
        &[
            owner.key.as_ref(),
            worker.key.as_ref(),
            contract_id.as_bytes().as_ref(),
        ],
        program_id,
    );

    validate_accounts_on_increment_step(program_id, owner, pda, &pda_key)?;
    increment_step(worker, pda)
}

fn increment_step(worker: &AccountInfo, pda: &AccountInfo) -> ProgramResult {
    
    let mut contract_data =
        try_from_slice_unchecked::<ContractData>(&pda.data.borrow())?;

    let transfer_amount = get_transfer_amount(contract_data.total_quantity, contract_data.actual_step)?;

    **worker.lamports.borrow_mut() = worker.lamports()
        .checked_add(transfer_amount)
        .ok_or(ProgramError::InsufficientFunds)?;

    **pda.lamports.borrow_mut() = pda.lamports()
        .checked_sub(transfer_amount)
        .ok_or(ProgramError::InsufficientFunds)?;

    msg!("{} lamports transferred from contract to {}", transfer_amount, worker.key);

    contract_data.actual_step += 1;
    contract_data.serialize(&mut &mut pda.data.borrow_mut()[..])?;

    Ok(())
}

fn validate_accounts_on_creation(
    owner: &AccountInfo,
    pda: &AccountInfo,
    pda_key: &Pubkey
) -> ProgramResult {

    if pda.key != pda_key {
        return Err(ProgramError::InvalidAccountData);
    }

    if !owner.is_signer {
        return Err(ProgramError::IllegalOwner);
    }

    Ok(())
}

fn validate_accounts_on_increment_step(
    program_id: &Pubkey,
    owner: &AccountInfo,
    pda: &AccountInfo,
    pda_key: &Pubkey
) -> ProgramResult {

    if pda.key != pda_key {
        return Err(ProgramError::InvalidAccountData);
    }

    if pda.owner != program_id {
        return Err(ProgramError::InvalidAccountData);
    }

    if !owner.is_signer {
        return Err(ProgramError::IllegalOwner);
    }

    Ok(())
}

fn get_transfer_amount(total_quantity: u64, actual_step: u64) -> Result<u64, ProgramError> {
    match actual_step {
        0 | 1 => Ok(total_quantity / 3),
        2 => {
            let quantity_per_three = total_quantity / 3;
            Ok(total_quantity - quantity_per_three - quantity_per_three)
        }
        _ => {
            Err(ProgramError::InsufficientFunds)
        }
    }
}
