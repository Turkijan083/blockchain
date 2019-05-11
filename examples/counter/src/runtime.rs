use primitive_types::H256;
use blockchain::traits::{
	Block as BlockT, BlockExecutor,
	BuilderExecutor, StorageExternalities,
};
use codec::{Encode, Decode};
use codec_derive::{Decode, Encode};
use sha3::{Digest, Sha3_256};

const DIFFICULTY: usize = 2;

fn is_all_zero(arr: &[u8]) -> bool {
	arr.iter().all(|i| *i == 0)
}

#[derive(Clone, Debug)]
pub struct UnsealedBlock {
	parent_hash: Option<H256>,
	extrinsics: Vec<Extrinsic>,
}

impl UnsealedBlock {
	pub fn seal(self) -> Block {
		let mut block = Block {
			parent_hash: self.parent_hash,
			extrinsics: self.extrinsics,
			nonce: 0,
		};

		while !is_all_zero(&block.id()[0..DIFFICULTY]) {
			block.nonce += 1;
		}

		block
	}
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Block {
	parent_hash: Option<H256>,
	extrinsics: Vec<Extrinsic>,
	nonce: u64,
}

impl Block {
	pub fn genesis() -> Self {
		Block {
			parent_hash: None,
			extrinsics: Vec::new(),
			nonce: 0,
		}
	}
}

impl BlockT for Block {
	type Identifier = H256;

	fn parent_id(&self) -> Option<H256> {
		self.parent_hash
	}

	fn id(&self) -> H256 {
		H256::from_slice(Sha3_256::digest(&self.encode()).as_slice())
	}
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum Extrinsic {
	Add(u128),
}

#[derive(Debug)]
pub enum Error {
	Backend(Box<std::error::Error>),
	DifficultyTooLow,
	StateCorruption,
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Error::DifficultyTooLow => "Difficulty too low".fmt(f)?,
			Error::StateCorruption => "State is corrupted".fmt(f)?,
			Error::Backend(_) => "Backend error".fmt(f)?,
		}

		Ok(())
	}
}

impl std::error::Error for Error { }

#[derive(Clone)]
pub struct Executor;

impl Executor {
	fn read_counter(&self, state: &mut <Self as BlockExecutor>::Externalities) -> Result<u128, Error> {
		Ok(
			match state.read_storage(b"counter").map_err(|e| Error::Backend(e))? {
				Some(counter) => {
					u128::decode(&mut counter.as_slice()).ok_or(Error::StateCorruption)?
				},
				None => 0,
			}
		)
	}

	fn write_counter(&self, counter: u128, state: &mut <Self as BlockExecutor>::Externalities) {
		state.write_storage(b"counter".to_vec(), counter.encode());
	}
}

impl BlockExecutor for Executor {
	type Error = Error;
	type Block = Block;
	type Externalities = dyn StorageExternalities + 'static;

	fn execute_block(
		&self,
		block: &Self::Block,
		state: &mut Self::Externalities,
	) -> Result<(), Error> {
		if !is_all_zero(&block.id()[0..DIFFICULTY]) {
			return Err(Error::DifficultyTooLow);
		}

		let mut counter = self.read_counter(state)?;

		for extrinsic in &block.extrinsics {
			match extrinsic {
				Extrinsic::Add(add) => counter += add,
			}
		}

		self.write_counter(counter, state);

		Ok(())
	}
}

impl BuilderExecutor for Executor {
	type Error = Error;
	type Block = Block;
	type BuildBlock = UnsealedBlock;
	type Externalities = dyn StorageExternalities + 'static;
	type Extrinsic = Extrinsic;
	type Inherent = ();

	fn initialize_block(
		&self,
		block: &Self::Block,
		_state: &mut Self::Externalities,
		_inherent: (),
	) -> Result<Self::BuildBlock, Self::Error> {
		Ok(UnsealedBlock {
			parent_hash: Some(block.id()),
			extrinsics: Vec::new(),
		})
	}

	fn apply_extrinsic(
		&self,
		_block: &mut Self::BuildBlock,
		extrinsic: Self::Extrinsic,
		state: &mut Self::Externalities,
	) -> Result<(), Self::Error> {
		let mut counter = self.read_counter(state)?;

		match extrinsic {
			Extrinsic::Add(add) => {
				counter += add;
			},
		}

		self.write_counter(counter, state);

		Ok(())
	}

	fn finalize_block(
		&self,
		_block: &mut Self::BuildBlock,
		_state: &mut Self::Externalities,
	) -> Result<(), Self::Error> {
		Ok(())
	}
}
