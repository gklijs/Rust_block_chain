extern crate time;
extern crate serde;
extern crate serde_json;
extern crate sha2;

use std::thread;
use self::sha2::{Sha256, Digest};
use std::fmt::Write;
use std::sync::mpsc::Sender;
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
struct Transaction {
    sender: String,
    receiver: String,  
    amount: f32,
}

#[derive(Serialize, Debug)]
pub struct Blockheader {
    timestamp: i64,
    nonce: i32,
    pre_hash: String,  
    merkle: String,  
    difficulty: i32,
}

#[derive(Serialize, Debug)]
pub struct Block {
    header: Blockheader,
    count: i32,
    transactions: Vec<Transaction>
}

pub struct Chain {
    chain: Vec<Block>,
    curr_trans: Vec<Transaction>,
    difficulty: i32,
    miner_addr: String, 
    reward: f32,
}

impl Chain {
    pub fn new(miner_addr: String, difficulty: i32, threads: i32) -> Chain {
        let mut chain = Chain {
            chain: Vec::new(),
            curr_trans: Vec::new(),
            difficulty,
            miner_addr,
            reward: 100.0,
        };

        chain.generate_new_block(threads);
        chain

    }

    pub fn new_transaction(&mut self, sender: String, receiver: String, amount: f32) -> bool {
        self.curr_trans.push(Transaction{
            sender,
            receiver,
            amount,
        });

        true
    }

    pub fn last_hash(&self) -> String {
        let block = match self.chain.last() {
            Some(block) => block,
            None => return String::from_utf8(vec![48; 64]).unwrap()
        };
        Chain::hash(&block.header)
    }

    pub fn update_difficulty(&mut self, difficulty: i32) -> bool {
        self.difficulty = difficulty;
        true
    }

    pub fn update_reward(&mut self, reward: f32) -> bool {
        self.reward = reward;
        true
    }

    pub fn generate_new_block(&mut self, threads: i32) -> bool {
        let reward_trans = Transaction {
            sender: String::from("Root"),
            receiver: self.miner_addr.clone(),
            amount: self.reward
        };
        let mut transactions = vec![reward_trans];
        transactions.append(&mut self.curr_trans);
        let count = transactions.len() as i32;
        let merkle = Chain::get_merkle(transactions.clone());
        let header = Chain::proof_of_work(self.last_hash(), self.difficulty, merkle, threads);

        let block = Block {
            header,
            count,
            transactions,
        };

        println!("{:#?}", block);
        self.chain.push(block);
        true
    }

    fn get_merkle(curr_trans: Vec<Transaction>) -> String {
        let mut merkle = Vec::new();

        for t in &curr_trans {
            let hash = Chain::hash(t);
            merkle.push(hash);
        }

        if merkle.len() % 2 == 1 {
            let last = merkle.last().cloned().unwrap();
            merkle.push(last);
        }

        while merkle.len() > 1 {
            let mut h1 = merkle.remove(0);
            let mut h2 = merkle.remove(0);
            h1.push_str(&mut h2);
            let nh = Chain::hash(&h1);
            merkle.push(nh);
        }
        merkle.pop().unwrap()
    }

    pub fn proof_of_work(pre_hash: String, difficulty: i32, merkle: String, threads: i32) -> Blockheader {
        let (sender, retriever) = mpsc::channel();
        let result_found  = Arc::new(AtomicBool::new(false));
        let timestamp = time::now().to_timespec().sec;
        for nonce in 0..threads{
            let mut header = Blockheader {
                timestamp,
                nonce,
                pre_hash: pre_hash.clone(),
                merkle: merkle.clone(),
                difficulty
            };
            let sender_n = sender.clone();
            let result_found_n = result_found.clone();
            thread::spawn(move || proof_thread(&mut header, threads, result_found_n, sender_n));
        }
        let nonce = retriever.recv().unwrap();
        Blockheader {
            timestamp,
            nonce,
            pre_hash: pre_hash.clone(),
            merkle: merkle.clone(),
            difficulty
        }
    }

    pub fn hash<T: serde::Serialize>(item: &T) -> String {
        let input = serde_json::to_string(&item).unwrap();
        //println!("Input for hash: {}", input);
        let mut hasher = Sha256::default();
        hasher.input(input.as_bytes());
        let res = hasher.result();
        let vec_res = res.to_vec();
        Chain::hex_to_string(vec_res.as_slice())
    }

    pub fn hex_to_string(vec_res: &[u8]) -> String {
        let mut s = String::new();
        for b in vec_res {
            write!(&mut s, "{:x}", b).expect("unable to write");
        }
        s
    }
}

fn proof_thread(header: &mut Blockheader, threads: i32, result_found: Arc<AtomicBool>, sender: Sender<i32>) {
    while !(*result_found).load(Ordering::Relaxed) {
        let hash = Chain::hash(header);
        let slice = &hash[..header.difficulty as usize];
        //println!("Tried nonce: {} resulted in slice: {}", header.nonce, slice);
        match slice.parse::<u32>() {
            Ok(val) => {
                if val != 0 {
                    header.nonce += threads;
                } else {
                    println!("Block hash: {}", hash);
                    break;
                }
            },
            Err(_) => {
                header.nonce += threads;
                continue;
            }
        };
    }
    (*result_found).store(true, Ordering::Relaxed);
    sender.send(header.nonce);
}