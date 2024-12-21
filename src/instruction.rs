use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum SocialInstruction {
    InitializeUser { seed_type: String },
    FollowUser { user_to_follow: Pubkey },
    UnfollowUser { user_to_unfollow: Pubkey },
    QueryFollower,
    PostContent { content: String },
    QueryPosts,
}
