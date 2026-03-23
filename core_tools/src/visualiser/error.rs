use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidTx,
    InvalidFixture,
    MissingPrevout,
    DuplicatePrevout,
    InvalidBlock,
    MerkleRootMismatch,
    InvalidUndoData,
    FileNotFound,
    NotImplemented,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidTx => "INVALID_TX",
            ErrorCode::InvalidFixture => "INVALID_FIXTURE",
            ErrorCode::MissingPrevout => "MISSING_PREVOUT",
            ErrorCode::DuplicatePrevout => "DUPLICATE_PREVOUT",
            ErrorCode::InvalidBlock => "INVALID_BLOCK",
            ErrorCode::MerkleRootMismatch => "MERKLE_ROOT_MISMATCH",
            ErrorCode::InvalidUndoData => "INVALID_UNDO_DATA",
            ErrorCode::FileNotFound => "FILE_NOT_FOUND",
            ErrorCode::NotImplemented => "NOT_IMPLEMENTED",
        }
    }
}

#[derive(Debug, Error)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl AppError {
    pub fn new(code: ErrorCode, msg: impl Into<String>) -> Self {
        AppError {
            code,
            message: msg.into(),
        }
    }

    pub fn invalid_tx(msg: impl Into<String>) -> Self {
        AppError::new(ErrorCode::InvalidTx, msg)
    }

    pub fn invalid_fixture(msg: impl Into<String>) -> Self {
        AppError::new(ErrorCode::InvalidFixture, msg)
    }

    pub fn missing_prevout(txid: &str, vout: u32) -> Self {
        AppError::new(
            ErrorCode::MissingPrevout,
            format!("No prevout found for input {}:{}", txid, vout),
        )
    }

    pub fn duplicate_prevout(txid: &str, vout: u32) -> Self {
        AppError::new(
            ErrorCode::DuplicatePrevout,
            format!("Duplicate prevout {}:{}", txid, vout),
        )
    }

    pub fn invalid_block(msg: impl Into<String>) -> Self {
        AppError::new(ErrorCode::InvalidBlock, msg)
    }

    pub fn merkle_mismatch(expected: &str, got: &str) -> Self {
        AppError::new(
            ErrorCode::MerkleRootMismatch,
            format!("Merkle root mismatch: header={} computed={}", expected, got),
        )
    }

    pub fn invalid_undo(msg: impl Into<String>) -> Self {
        AppError::new(ErrorCode::InvalidUndoData, msg)
    }
}
