use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    instruction::*,
    state::{Post, UserPost, UserProfile},
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh1::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
const PUBKEY_SIZE: usize = 32;
const U16_SIZE: usize = 2;
const U64_SIZE: usize = 8;
const USER_PROFILE_SIZE: usize = 6;
const USER_POST_SIZE: usize = 12;
const MAX_FOLLOWER_COUNT: usize = 200;
const FIXED_CONRTENT_LEN: usize = 20; //this is hardcoded to 20 for easier implementation
const MAX_POST_COUNT: usize = 100;
const TIMESTAMP_SIZE: usize = 8;

pub struct Processor;

impl Processor {
    pub fn processs_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = SocialInstruction::try_from_slice(instruction_data)?;
        match instruction {
            SocialInstruction::InitializeUser { seed_type } => {
                Self::initialize_user(program_id, accounts, seed_type)
            }
            SocialInstruction::FollowUser { user_to_follow } => {
                Self::follow_user(accounts, user_to_follow)
            }
            SocialInstruction::UnfollowUser { user_to_unfollow } => {
                Self::unfollow_user(accounts, user_to_unfollow)
            }
            SocialInstruction::QueryFollower => Self::query_followers(accounts),
            SocialInstruction::PostContent { content } => Self::post_content(accounts, content),
            SocialInstruction::QueryPosts => Self::query_post(accounts),
        }
    }
    fn initialize_user(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        seed_type: String,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user_account = next_account_info(account_info_iter)?;
        let pda_account = next_account_info(account_info_iter)?;
        let _system_program = next_account_info(account_info_iter)?;

        let seed: &str = match seed_type.as_str() {
            "profile" => "profile",
            "post" => "post",
            _ => return Err(ProgramError::InvalidArgument),
        };

        msg!("seed is {}", seed);

        let (pda, bump_seed) =
            Pubkey::find_program_address(&[user_account.key.as_ref(), seed.as_bytes()], program_id);

        msg!("pda is {}", pda);

        if pda != pda_account.key.clone() {
            return Err(ProgramError::InvalidArgument);
        }

        let rent: Rent = Rent::get()?;
        let space = match seed_type.as_str() {
            "profile" => compute_profile_space(MAX_FOLLOWER_COUNT),
            "post" => compute_post_space(MAX_POST_COUNT),
            _ => return Err(ProgramError::InvalidArgument),
        };

        let lamports = rent.minimum_balance(space);
        let create_account_ix = system_instruction::create_account(
            user_account.key,
            &pda,
            lamports,
            space as u64,
            program_id,
        );
        invoke_signed(
            &create_account_ix,
            accounts,
            &[&[user_account.key.as_ref(), seed.as_bytes(), &[bump_seed]]],
        )?;

        match seed_type.as_str() {
            "profile" => {
                let user_profile = UserProfile::new();
                user_profile.serialize(&mut *pda_account.try_borrow_mut_data()?)?;
            }
            "post" => {
                let user_post = UserPost::new();
                user_post.serialize(&mut *pda_account.try_borrow_mut_data()?)?;
            }
            _ => return Err(ProgramError::InvalidArgument),
        };

        Ok(())
    }
    fn follow_user(accounts: &[AccountInfo], user: Pubkey) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pad_account: &AccountInfo<'_> = next_account_info(account_info_iter)?;

        let mut size = 0;
        {
            let data = &pad_account.data.borrow();
            let len = &data[..U16_SIZE];
            let pubkey_count = bytes_to_u16(len).unwrap();
            size = compute_profile_space(pubkey_count as usize);
            msg!("size is {:?}", size)
        }
        let mut user_profile = UserProfile::try_from_slice(&pad_account.data.borrow()[..size])?;
        msg!("user_profile is {:?}", user_profile);
        user_profile.follow(user);
        user_profile.serialize(&mut *pad_account.try_borrow_mut_data()?)?;

        Ok(())
    }

    fn unfollow_user(accounts: &[AccountInfo], user_to_unfollow: Pubkey) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pda_account = next_account_info(account_info_iter)?;
        let mut size = 0;
        {
            let data = &pda_account.data.borrow();
            let len = &data[..U16_SIZE];
            let pubkey_count = bytes_to_u16(len).unwrap();
            size = compute_profile_space(pubkey_count as usize);
            msg!("size is {:?}", size)
        }
        let mut user_profile = UserProfile::try_from_slice(&pda_account.data.borrow()[..size])?;
        msg!("user_profile is {:?}", user_profile);
        user_profile.un_follow(user_to_unfollow);
        user_profile.serialize(&mut *pda_account.try_borrow_mut_data()?)?;
        Ok(())
    }
    fn query_followers(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pda_account = next_account_info(account_info_iter)?;
        let user_profile =
            try_from_slice_unchecked::<UserProfile>(&pda_account.data.borrow()).unwrap();
        msg!("usef_profile is {:?}", user_profile);
        Ok(())
    }
    fn post_content(accounts: &[AccountInfo], content: String) -> ProgramResult {
        if content.len() != FIXED_CONRTENT_LEN {
            return Err(ProgramError::InvalidArgument);
        }
        let account_info_iter = &mut accounts.iter();
        //let user_account = next_account_info(account_info_iter)?;
        let pda_account = next_account_info(account_info_iter)?;
        //let _system_program = next_account_info(account_info_iter)?;
        let mut size = 0;
        {
            let data = &pda_account.data.borrow();
            let len = &data[..U64_SIZE];
            let post_count = bytes_to_u64(len).unwrap();
            size = compute_post_space(post_count as usize);
            msg!("size is {:?}", size)
        }
        let mut user_posts: UserPost =
            UserPost::try_from_slice(&pda_account.data.borrow()[..size])?;
        let timestamp = user_posts.post_count + 1;
        let post = Post::new(content, timestamp);
        user_posts.post(post);
        msg!("user_posts is {:?}", user_posts);
        user_posts.serialize(&mut *pda_account.try_borrow_mut_data()?)?;
        Ok(())
    }
    fn query_post(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        //let user_account = next_account_info(account_info_iter)?;
        let pda_account = next_account_info(account_info_iter)?;
        //let _system_program = next_account_info(account_info_iter)?;
        let mut size = 0;
        {
            let data = &pda_account.data.borrow();
            let len = &data[..U64_SIZE];
            let post_count = bytes_to_u64(len).unwrap();
            size = compute_post_space(post_count as usize);
            msg!("size is {:?}", size)
        }
        let user_posts = UserPost::try_from_slice(&pda_account.data.borrow()[..size])?;
        msg!("user_posts is {:?}", user_posts);
        Ok(())
    }
}
fn compute_profile_space(pubkey_count: usize) -> usize {
    return USER_PROFILE_SIZE + pubkey_count * PUBKEY_SIZE;
}

fn compute_post_space(post_count: usize) -> usize {
    return USER_POST_SIZE + (4 + FIXED_CONRTENT_LEN + TIMESTAMP_SIZE) * post_count;
}
fn bytes_to_u16(bytes: &[u8]) -> Option<u16> {
    if bytes.len() != 2 {
        return None;
    }
    let mut array = [0u8; 2];
    array.copy_from_slice(bytes);
    Some(u16::from_le_bytes(array))
}

fn bytes_to_u64(bytes: &[u8]) -> Option<u64> {
    if bytes.len() != 8 {
        return None;
    }
    let mut array = [0u8; 8];
    array.copy_from_slice(bytes);
    Some(u64::from_le_bytes(array))
}
