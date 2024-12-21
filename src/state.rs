use std::thread::sleep;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct UserProfile {
    pub data_len: u16,
    pub follows: Vec<Pubkey>,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct UserPost {
    pub post_count: u64,
    pub posts: Vec<Post>,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Post {
    pub content: String,
    pub timestamp: u64,
}

impl UserProfile {
    pub fn new() -> Self {
        Self {
            data_len: 0,
            follows: Vec::new(),
        }
    }

    pub fn follow(&mut self, user: Pubkey) {
        self.follows.push(user);
        self.data_len = self.follows.len() as u16;
    }

    pub fn un_follow(&mut self, user: Pubkey) {
        self.follows.retain(|&x| x != user);
        self.data_len = self.follows.len() as u16;
    }
}
impl Post {
    pub fn new(content: String, timestamp: u64) -> Self {
        Self { content, timestamp }
    }
}
impl UserPost {
    pub fn new() -> Self {
        Self {
            post_count: 0,
            posts: Vec::new(),
        }
    }
    pub fn post(&mut self, post: Post) {
        self.posts.push(post);
        self.post_count = self.posts.len() as u64;
    }
    pub fn query_posts(&self) -> &Vec<Post> {
        self.posts.as_ref()
    }
}
