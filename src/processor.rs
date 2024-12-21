use crate::{
    instruction::*,
    state::{Post, UserPost, UserProfile},
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh1::try_from_slice_unchecked,
    clock::Clock,
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
const USER_PROFILE_SIZE: usize = 6;
const USER_POST_SIZE: usize = 8;
const MAX_FOLLOWER_COUNT: usize = 200;

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
            SocialInstruction::PostContent { content } => {
                Self::post_content(program_id, accounts, content)
            }
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
            "post" => USER_POST_SIZE,
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
        //with try_from_slice_unchecked, size calculation is not required.
        let mut user_profile = try_from_slice_unchecked::<UserProfile>(&pda_account.data.borrow())?;
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
    fn post_content(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        content: String,
    ) -> ProgramResult {
        let account_info_iter: &mut std::slice::Iter<'_, AccountInfo<'_>> = &mut accounts.iter();
        let user_account = next_account_info(account_info_iter)?;
        let pda_account = next_account_info(account_info_iter)?;
        let pda_account_post = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        let mut user_posts =
            try_from_slice_unchecked::<UserPost>(&pda_account.data.borrow()).unwrap();
        user_posts.add_post();
        let count = user_posts.get_count();
        msg!("post count is {}", count);
        user_posts.serialize(&mut *pda_account.try_borrow_mut_data()?)?;

        let (pda, bump_seed) = Pubkey::find_program_address(
            &[
                user_account.key.as_ref(),
                "post".as_bytes(),
                &count.to_le_bytes(),
            ],
            program_id,
        );
        msg!("post pda is {}", pda);

        if pda != pda_account_post.key.clone() {
            return Err(ProgramError::InvalidArgument);
        }
        let clock = Clock::get()?;
        let timestamp = clock.unix_timestamp as u64;
        let post = Post::new(content, timestamp);

        let rent: Rent = Rent::get()?;
        let space = borsh::to_vec(&post).unwrap().len();

        msg!("Space: {}", space);

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
            &[
                user_account.clone(),
                pda_account_post.clone(),
                system_program.clone(),
            ],
            &[&[
                user_account.key.as_ref(),
                "post".as_bytes(),
                &count.to_le_bytes(),
                &[bump_seed],
            ]],
        )?;
        msg!("post pda created {}", pda);

        post.serialize(&mut *pda_account_post.try_borrow_mut_data()?)?;

        Ok(())
    }
    fn query_post(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        //let user_account = next_account_info(account_info_iter)?;
        let pda_account = next_account_info(account_info_iter)?;

        let post: Post = try_from_slice_unchecked::<Post>(&pda_account.data.borrow())?;
        msg!("post is {:?}", post);
        Ok(())
    }
}
fn compute_profile_space(pubkey_count: usize) -> usize {
    return USER_PROFILE_SIZE + pubkey_count * PUBKEY_SIZE;
}

fn bytes_to_u16(bytes: &[u8]) -> Option<u16> {
    if bytes.len() != 2 {
        return None;
    }
    let mut array = [0u8; 2];
    array.copy_from_slice(bytes);
    Some(u16::from_le_bytes(array))
}
