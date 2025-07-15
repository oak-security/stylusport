<PART 1 - STRUCTURED SOLANA DOCS>
>>> prompts/structured-solana-docs/programs.mdx
# SOLANA PROGRAMS

## CORE CONCEPTS
- Programs = smart contracts deployed to executable accounts
- Users interact via transactions containing instructions
- Programs are stateless but can create/update data accounts
- Upgrade authority can update programs until it's removed
- Verifiable builds allow source code verification

## DEVELOPMENT APPROACHES
1. Anchor Framework (recommended for beginners)
   - Faster, simpler development
   - Rust macros reduce boilerplate
   - Built-in features (PDAs, CPIs, security)

2. Native Rust
   - More flexibility, greater complexity
   - Full control over implementation

## PROGRAM UPDATES
- Programs can be modified by designated upgrade authority
- Removing upgrade authority makes program immutable
- Deploy/upgrade details: See deploying programs page

## VERIFIABLE PROGRAMS
- Verify on-chain code matches public source
- Tools:
  - Solana Explorer (quickly check verification)
  - Solana Verifiable Build CLI
  - Anchor's built-in verification support

## TECHNICAL IMPLEMENTATION
- LLVM compiles to ELF files containing sBPF bytecode
- ELF binary stored in executable account on-chain

## PROGRAM TYPES

### LOADER PROGRAMS
- Every program owned by a loader program
- Five loaders: native, v1, v2, v3, v4
- Manage program deployment, upgrades, authority

### PRECOMPILED PROGRAMS
- **Ed25519**: Signature verification
- **Secp256k1**: Public key recovery operations
- **Secp256r1**: Signature verification

### CORE PROGRAMS
- **System Program**: Account creation, SOL transfers
- **Vote Program**: Validator voting state and rewards
- **Stake Program**: Stake delegation and rewards
- **Config Program**: On-chain configuration
- **Compute Budget Program**: Transaction resource control
- **Address Lookup Table Program**: Extended account references
- **ZK ElGamal Proof Program**: Zero-knowledge proofs
>>> prompts/structured-solana-docs/program-structure.mdx
# RUST PROGRAM STRUCTURE

## ORGANIZATION
Typical file structure (flexible):
- entrypoint.rs: Defines program entry point
- state.rs: Program-specific account data structures
- instructions.rs: Program operations
- processor.rs: Instruction handler implementation
- error.rs: Custom error definitions

## ENTRYPOINT
- Required component where execution begins
- Defined using `entrypoint!` macro
- Handles raw input from runtime
- Parameters:
  - program_id: Public key of current program
  - accounts: AccountInfo array for data access
  - instruction_data: Instruction identifier and arguments

## STATE MANAGEMENT
- Define program state using Rust structs
- Implement serialization/deserialization (typically Borsh)
- Account state structure:
  ```rust
  #[derive(BorshSerialize, BorshDeserialize, Debug)]
  pub struct CounterAccount {
      count: u64,
  }
  ```

## INSTRUCTION DEFINITION
- Define program operations using Rust enum
- Each variant = one instruction
- Implement parsing from raw bytes:
  ```rust
  #[derive(BorshSerialize, BorshDeserialize, Debug)]
  pub enum CounterInstruction {
      InitializeCounter { initial_value: u64 }, // variant 0
      IncrementCounter,                         // variant 1
  }
  ```

## INSTRUCTION HANDLERS
- Functions implementing instruction logic
- Account validation and error handling
- Data manipulation through serialization/deserialization
- Common pattern: `process_<instruction_name>` functions

## COUNTER PROGRAM EXAMPLE
Complete example implementing:
1. InitializeCounter: Creates account with initial value
2. IncrementCounter: Updates existing counter value

Key implementation aspects:
- Account creation (System Program CPI)
- Rent calculation for account space
- Data serialization/deserialization
- Ownership verification
- Safe arithmetic operations
>>> prompts/structured-solana-docs/pda.mdx
# PROGRAM DERIVED ADDRESS (PDA)

## USE CASES
- Deterministic account addresses: Create predictable addresses from inputs
- Program signing: Runtime enables programs to "sign" for PDAs derived from program's address
- On-chain hashmaps: Create structured data relationships using seed derivation

## KEY POINTS
- PDAs are deterministically derived from seeds, bump, and program ID
- PDAs fall off Ed25519 curve (have no private key)
- Programs can sign for PDAs derived from their program ID
- Deriving a PDA doesn't create an account - explicit creation required
- PDA accounts must be created through the program used to derive the address

## PDA DERIVATION
- Inputs:
  1. Optional seeds: Strings, numbers, account addresses as predefined inputs
  2. Bump seed: Byte (starts at 255, decrements until valid off-curve address found)
  3. Program ID: Address of program that can sign for this PDA
- SDK Functions:
  - TypeScript: getProgramDerivedAddress (@solana/kit)
  - TypeScript: findProgramAddressSync (@solana/web3.js)
  - Rust: find_program_address (solana_sdk)

## CANONICAL BUMP
- First bump value (starting from 255) that produces valid off-curve address
- Security: Always check PDAs are derived with canonical bump to prevent vulnerabilities

## CREATING PDA ACCOUNTS
- Process:
  1. Derive PDA address with program ID and seeds
  2. Invoke System Program to create account at PDA address
  3. Initialize account data through program
- In Anchor:
  ```rust
  #[account(
      init,
      seeds = [b"data", user.key().as_ref()],
      bump,
      payer = user,
      space = 8 + DataAccount::INIT_SPACE
  )]
  pub pda_account: Account<'info, DataAccount>,
  ```
>>> prompts/structured-solana-docs/cpi.mdx
# CROSS PROGRAM INVOCATION (CPI)

## DEFINITION
- When one program invokes instructions of another program
- Enables composability of Solana programs
- Instructions as API endpoints; CPIs as internal API calls

## KEY POINTS
- Allows direct invocation of other program instructions
- Signer privileges extend from caller to callee program
- Programs can sign on behalf of PDAs derived from their program ID
- Max CPI depth of 4 programs (A→B→C→D)

## CPI COMPONENTS
- Program address: Specifies program to invoke
- Accounts: Lists accounts to read/write
- Instruction data: Specifies which instruction to invoke + arguments

## INVOKE FUNCTION
- For CPIs without PDA signers
- Function signature:
  ```rust
  pub fn invoke(
    instruction: &Instruction, 
    account_infos: &[AccountInfo]
  ) -> ProgramResult
  ```

## INVOKE_SIGNED FUNCTION
- For CPIs requiring PDA signers
- Function signature:
  ```rust
  pub fn invoke_signed(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    signers_seeds: &[&[&[u8]]]
  ) -> ProgramResult
  ```
- Runtime creates valid PDAs using signers_seeds and program_id

## IMPLEMENTATION APPROACHES
1. High-level Anchor CpiContext + helper functions
2. Mid-level using specialized instruction builders (system_instruction::transfer)
3. Low-level manual instruction construction
>>> prompts/structured-solana-docs/accounts.mdx
# SOLANA ACCOUNT MODEL

## CORE CONCEPTS
- Accounts: Key-value database storage units (max 10MiB)
- Rent: Deposit proportional to data size (fully recoverable when closing account)
- Ownership: Only owner program can modify data or deduct lamports
- Account Types: Sysvar accounts (network state), Program accounts (executable code), Data accounts (program state)

## ACCOUNT STRUCTURE
- Address: Unique 32-byte identifier (typically Ed25519 public key or PDA)
- Fields:
  - data: Byte array for arbitrary data or executable code
  - executable: Flag indicating if account is a program
  - lamports: Balance (1 SOL = 1 billion lamports)
  - owner: Program ID that controls this account
  - rent_epoch: Legacy field (no longer used)

## SYSTEM PROGRAM
- Default owner of new accounts
- Key functions:
  - Create new accounts
  - Allocate data space
  - Transfer/assign program ownership
  - "Wallet" accounts are just System Program-owned accounts with SOL

## PROGRAM ACCOUNTS
- Store executable program code
- Owned by a Loader Program
- Address (Program ID) used when invoking instructions

## DATA ACCOUNTS
- Store program state
- Creation process:
  1. System Program creates account
  2. Ownership transferred to custom program
  3. Custom program initializes account data

## PROGRAM DERIVED ADDRESSES (PDAs)
- Deterministic addresses derived from program ID and optional inputs
- Off-curve addresses (no private key)
- Programs can sign for PDAs derived from their ID
</PART1>
<PART 2 - STRUCTURED ANCHOR DOCS>
>>> prompts/structured-anchor-docs/account-constraints.mdx
# ANCHOR ACCOUNT CONSTRAINTS

## NORMAL CONSTRAINTS

### SIGNER
- **Purpose**: Checks if account signed the transaction
- **Syntax**: `#[account(signer)]` or `#[account(signer @ <custom_error>)]`
- **Note**: Consider using `Signer<'info>` type instead when this is the only constraint

### MUT
- **Purpose**: Marks account as mutable and persists state changes
- **Syntax**: `#[account(mut)]` or `#[account(mut @ <custom_error>)]`

### INIT
- **Purpose**: Creates account via System Program CPI and initializes it
- **Syntax**: 
  ```rust
  #[account(init, payer = <target_account>, space = <num_bytes>)]
  ```

### INIT_IF_NEEDED
- **Purpose**: Like init but only runs if account doesn't exist
- **Syntax**: 
  ```rust
  #[account(init_if_needed, payer = <target_account>, space = <num_bytes>)]
  ```

### SEEDS AND BUMP
- **Purpose**: Validates PDA derived from program ID, seeds, and bump
- **Syntax**:
  ```rust
  #[account(seeds = <seeds>, bump)]
  #[account(seeds = <seeds>, bump, seeds::program = <expr>)]
  #[account(seeds = <seeds>, bump = <expr>)]
  ```

### HAS_ONE
- **Purpose**: Validates account field matches target account key
- **Syntax**: 
  ```rust
  #[account(has_one = <target_account>)]
  #[account(has_one = <target_account> @ <custom_error>)]
  ```

### ADDRESS
- **Purpose**: Validates account key matches specific pubkey
- **Syntax**: 
  ```rust
  #[account(address = <expr>)]
  #[account(address = <expr> @ <custom_error>)]
  ```

### OWNER
- **Purpose**: Validates account owner matches specific program
- **Syntax**: 
  ```rust
  #[account(owner = <expr>)]
  #[account(owner = <expr> @ <custom_error>)]
  ```

### EXECUTABLE
- **Purpose**: Validates account is a program (executable)
- **Syntax**: `#[account(executable)]`

### ZERO
- **Purpose**: Validates account discriminator is zero (for large accounts)
- **Syntax**: `#[account(zero)]`

### CLOSE
- **Purpose**: Closes account by transferring lamports and resetting data
- **Syntax**: `#[account(close = <target_account>)]`

### CONSTRAINT
- **Purpose**: Custom validation that expression evaluates to true
- **Syntax**: 
  ```rust
  #[account(constraint = <expr>)]
  #[account(constraint = <expr> @ <custom_error>)]
  ```

### REALLOC
- **Purpose**: Reallocates account space during instruction execution
- **Syntax**:
  ```rust
  #[account(realloc = <space>, realloc::payer = <target>, realloc::zero = <bool>)]
  ```

## SPL CONSTRAINTS

### TOKEN
- **Purpose**: Validates token account mint and authority
- **Syntax**:
  ```rust
  #[account(token::mint = <target_account>, token::authority = <target_account>)]
  ```

### MINT
- **Purpose**: Validates mint account parameters
- **Syntax**:
  ```rust
  #[account(mint::authority = <target_account>, mint::decimals = <expr>)]
  ```

### ASSOCIATED_TOKEN
- **Purpose**: Validates associated token account relationship
- **Syntax**:
  ```rust
  #[account(associated_token::mint = <target>, associated_token::authority = <target>)]
  ```

### TOKEN_PROGRAM
- **Purpose**: Overrides default token program
- **Syntax**: `#[account(*::token_program = <target_account>)]`

## INSTRUCTION ATTRIBUTE

### #[INSTRUCTION(...)]
- **Purpose**: Access instruction arguments within accounts struct
- **Syntax**: `#[instruction(arg1: Type1, arg2: Type2, ...)]`
- **Rules**:
  - List arguments in same order as instruction handler
  - Can omit trailing arguments but not skip middle ones
  - Used for dynamic space calculation or validation
>>> prompts/structured-anchor-docs/events.mdx
# ANCHOR EVENTS

## OVERVIEW
- Two methods for emitting events in Anchor programs:
  1. `emit!()`: Direct program logs (simpler)
  2. `emit_cpi!()`: Cross Program Invocation-based (more robust)

## EMIT! MACRO
- Uses `sol_log_data()` syscall to write data to program logs
- Encodes event data as base64 string prefixed with "Program Data:"
- Less compute-intensive but can be truncated by RPC providers

### Implementation
```rust
#[event]
pub struct CustomEvent {
    pub message: String,
}

pub fn emit_event(_ctx: Context<EmitEvent>, input: String) -> Result<()> {
    emit!(CustomEvent { message: input });
    Ok(())
}
```

### Client Usage
```typescript
const listenerId = program.addEventListener("customEvent", event => {
    console.log("Event Data:", event);
});

await program.methods.emitEvent(message).rpc();

await program.removeEventListener(listenerId);
```

## EMIT_CPI! MACRO
- Emits events through CPIs to program itself
- Event data included in instruction data, not program logs
- More robust against log truncation but uses more compute units
- Requires enabling `event-cpi` feature in Cargo.toml

### Cargo Configuration
```toml
[dependencies]
anchor-lang = { version = "0.31.1", features = ["event-cpi"] }
```

### Implementation
```rust
#[event]
pub struct CustomEvent {
    pub message: String,
}

#[event_cpi]  // Required for emit_cpi!
#[derive(Accounts)]
pub struct EmitEvent {}

pub fn emit_event(ctx: Context<EmitEvent>, input: String) -> Result<()> {
    emit_cpi!(CustomEvent { message: input });
    Ok(())
}
```

### Client Usage
```typescript
// Send transaction
const transactionSignature = await program.methods.emitEvent(message).rpc();

// Fetch transaction data
const transactionData = await program.provider.connection.getTransaction(
    transactionSignature, 
    { commitment: "confirmed" }
);

// Decode event from CPI instruction data
const eventIx = transactionData.meta.innerInstructions[0].instructions[0];
const rawData = anchor.utils.bytes.bs58.decode(eventIx.data);
const base64Data = anchor.utils.bytes.base64.encode(rawData.subarray(8));
const event = program.coder.events.decode(base64Data);
```

## NOTES
- For robust event solutions, consider geyser gRPC services (Triton or Helius)
- `emit_cpi!()` events cannot be subscribed to directly, must be manually decoded
>>> prompts/structured-anchor-docs/transfer-tokens.mdx
# TRANSFERRING TOKENS IN ANCHOR

## OVERVIEW
- DEFINITION: Moving tokens between token accounts that share the same mint
- REQUIREMENT: Only the owner of the source token account can transfer tokens
- COMPATIBILITY: Works with both Token Program and Token Extension Program

## IMPLEMENTATION APPROACH

### REQUIRED IMPORTS
```rust
use anchor_spl::token_interface::{
    self, Mint, TokenAccount, TokenInterface, TransferChecked
};
```

### ACCOUNT STRUCTURE
```rust
#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,                            // Source token account owner
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,              // Token mint
    #[account(mut)]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,    // Source
    #[account(mut)]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>, // Destination
    pub token_program: Interface<'info, TokenInterface>,  // Token program
}
```

### TRANSFER LOGIC
```rust
pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
    // Get the number of decimals for the mint
    let decimals = ctx.accounts.mint.decimals;
    
    // Create accounts struct for the CPI
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.sender_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
    };
    
    // Get token program account
    let cpi_program = ctx.accounts.token_program.to_account_info();
    
    // Create CPI context
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
    
    // Make the CPI to transfer tokens
    token_interface::transfer_checked(cpi_context, amount, decimals)?;
    Ok(())
}
```

## ADVANCED PATTERN: PDA AS TOKEN OWNER

### TOKEN ACCOUNT SETUP WITH PDA OWNER
```rust
#[derive(Accounts)]
pub struct CreateAndMintTokens<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 6,
        mint::authority = mint,
        mint::freeze_authority = mint,
        seeds = [b"mint"],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = signer,
        token::mint = mint,
        token::authority = token_account,  // Self-authority pattern
        seeds = [b"token"],
        bump
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```

### TRANSFERRING WITH PDA SIGNING
```rust
pub fn transfer_tokens(ctx: Context<TransferTokens>) -> Result<()> {
    // Create seeds array for PDA signing
    let signer_seeds: &[&[&[u8]]] = &[&[b"token", &[ctx.bumps.sender_token_account]]];

    // Get token amount and decimals
    let amount = ctx.accounts.sender_token_account.amount;  // Transfer all tokens
    let decimals = ctx.accounts.mint.decimals;
    
    // Create accounts struct for the CPI
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.sender_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: ctx.accounts.sender_token_account.to_account_info(), // PDA as authority
    };
    
    // Get token program account
    let cpi_program = ctx.accounts.token_program.to_account_info();
    
    // Create CPI context with PDA signer
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts)
        .with_signer(signer_seeds);
    
    // Make the CPI to transfer tokens
    token_interface::transfer_checked(cpi_context, amount, decimals)?;
    Ok(())
}
```

### ACCOUNT VALIDATION FOR PDA TRANSFER
```rust
#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"mint"],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = sender_token_account,
        seeds = [b"token"],
        bump
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program,
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
```

## CLIENT IMPLEMENTATION
```typescript
// Find PDAs
const [mint, mintBump] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("mint")],
  program.programId,
);

const [token, tokenBump] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("token")],
  program.programId,
);

// Transfer tokens
await program.methods
  .transferTokens()
  .accounts({
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  })
  .rpc();

// Get account info
const associatedTokenAccount = await getAssociatedTokenAddress(
  mint,
  program.provider.publicKey,
  false,
  TOKEN_2022_PROGRAM_ID,
);

const recipientTokenAccount = await getAccount(
  program.provider.connection,
  associatedTokenAccount,
  "confirmed",
  TOKEN_2022_PROGRAM_ID,
);

const senderTokenAccount = await getAccount(
  program.provider.connection,
  token,
  "confirmed",
  TOKEN_2022_PROGRAM_ID,
);
```

## KEY CONCEPTS
- TRANSFER CHECKED: Includes mint and decimals check for safety
- PDA SIGNING: Use `.with_signer(signer_seeds)` with PDA token authority
- SELF-AUTHORITY: Same PDA can be both token account address and token owner
- DECIMALS: Required parameter to verify correct token type and amount
>>> prompts/structured-anchor-docs/program-structure.mdx
# ANCHOR PROGRAM STRUCTURE

## KEY MACROS
- `declare_id!`: Specifies program's on-chain address
- `#[program]`: Defines module containing instruction logic
- `#[derive(Accounts)]`: Defines required accounts for instructions
- `#[account]`: Creates custom account data types

## BASIC STRUCTURE
```rust
use anchor_lang::prelude::*;

declare_id!("PROGRAM_ID_HERE");

#[program]
mod program_name {
    use super::*;
    pub fn instruction_name(ctx: Context<AccountsStruct>, args: Type) -> Result<()> {
        // Instruction logic
        Ok(())
    }
}

#[derive(Accounts)]
pub struct AccountsStruct<'info> {
    #[account(...constraints...)]
    pub account_name: AccountType<'info, DataType>,
}

#[account]
pub struct DataType {
    // Account data fields
}
```

## DECLARE_ID! MACRO
- Specifies on-chain program address
- Default: keypair public key at `/target/deploy/program_name.json`
- Update with `anchor keys sync`

## #[PROGRAM] ATTRIBUTE
- Annotates module containing instruction handlers
- Each public function = executable instruction
- First parameter: `Context<T>` with accounts validation
- Additional parameters: Optional instruction arguments

### CONTEXT TYPE
- `ctx.accounts`: Validated accounts for instruction
- `ctx.program_id`: Program's public key 
- `ctx.remaining_accounts`: Additional accounts not in struct
- `ctx.bumps`: PDA bump seeds from validation

## #[DERIVE(ACCOUNTS)] MACRO
- Defines accounts required by instruction
- Implements `Accounts` trait for validation/deserialization
- Each field = one required account
- Two validation methods:
  1. Account constraints: `#[account(...)]` attributes
  2. Account types: Type safety for different account roles

## #[ACCOUNT] ATTRIBUTE
- Creates custom account data structures
- Key functionalities:
  - Assigns program owner automatically
  - Adds 8-byte discriminator to account data
  - Handles serialization/deserialization

### ACCOUNT DISCRIMINATOR
- Unique 8-byte identifier for each account type
- First 8 bytes of SHA256 hash of `"account:<AccountName>"`
- Stored as first 8 bytes of account data
- Used for validation during deserialization
- Requires 8 bytes in account space allocation
>>> prompts/structured-anchor-docs/space.mdx
# ACCOUNT SPACE CALCULATION

## OVERVIEW
- Used to determine storage requirements for Anchor accounts
- Applies to standard accounts (not zero-copy accounts)
- Always add 8 bytes for Anchor's discriminator

## TYPE SIZE REFERENCE

| TYPE           | BYTES                        | NOTES                                     |
|----------------|------------------------------|-------------------------------------------|
| bool           | 1                            | Uses full byte despite needing only 1 bit |
| u8/i8          | 1                            |                                           |
| u16/i16        | 2                            |                                           |
| u32/i32        | 4                            |                                           |
| u64/i64        | 8                            |                                           |
| u128/i128      | 16                           |                                           |
| [T;amount]     | space(T) * amount            | Fixed-size arrays                         |
| Pubkey         | 32                           |                                           |
| Vec\<T>        | 4 + (space(T) * amount)      | 4 bytes for length prefix                 |
| String         | 4 + length in bytes          | 4 bytes for length prefix                 |
| Option\<T>     | 1 + space(T)                 | 1 byte for variant tag                    |
| Enum           | 1 + largest variant size     | 1 byte for variant tag                    |
| f32            | 4                            | NaN values cause serialization failure    |
| f64            | 8                            | NaN values cause serialization failure    |

## MANUAL CALCULATION EXAMPLE

```rust
#[account]
pub struct MyData {
    pub val: u16,                 // 2 bytes
    pub state: GameState,         // 1 + 32 bytes (enum with Pubkey in largest variant)
    pub players: Vec<Pubkey>      // 4 + (10 * 32) bytes (supports up to 10 players)
}

impl MyData {
    pub const MAX_SIZE: usize = 2 + (1 + 32) + (4 + 10 * 32);
}

// Usage in account initialization
#[account(init, payer = signer, space = 8 + MyData::MAX_SIZE)]
pub acc: Account<'info, MyData>,
```

## INITSPACE MACRO

### USAGE
- Automatically calculates account space requirements
- Adds `INIT_SPACE` constant to the structure
- Compatible with or without the `#[account]` attribute

```rust
#[account]
#[derive(InitSpace)]
pub struct ExampleAccount {
    pub data: u64,
    #[max_len(50)]              // String with max 50 chars
    pub string_one: String,
    #[max_len(10, 5)]           // Vec with max 10 elements, each a Vec with max 5 elements
    pub nested: Vec<Vec<u8>>,
}

// Usage in account initialization
#[account(init, payer = payer, space = 8 + ExampleAccount::INIT_SPACE)]
pub data: Account<'info, ExampleAccount>,
```

### VECTOR SIZE ANNOTATION
- `#[max_len(n)]`: Specifies maximum number of elements (not bytes)
- For nested vectors, use multiple arguments: `#[max_len(outer_count, inner_count)]`
- Total size calculation:
  - `Vec<T>`: 4 + (max_len * size_of::<T>())
  - `Vec<Vec<T>>`: 4 + (outer_max_len * (4 + (inner_max_len * size_of::<T>())))

## IMPORTANT NOTES
- Always add 8 bytes for Anchor's discriminator when initializing accounts
- Zero-copy accounts use C layout rules instead of these calculations
- For Vec/String, max_len specifies element count, not bytes
- Consider future expansion needs when allocating space
>>> prompts/structured-anchor-docs/mint-tokens.mdx
# MINTING TOKENS IN ANCHOR

## OVERVIEW
- DEFINITION: Creating new units of a token by invoking the `mint_to` instruction
- REQUIREMENT: Only the designated mint authority can mint new tokens
- COMPATIBILITY: Works with both Token Program and Token Extension Program

## IMPLEMENTATION APPROACH

### REQUIRED IMPORTS
```rust
use anchor_spl::{
    token_interface::{self, Mint, MintTo, TokenAccount, TokenInterface},
};
```

### ACCOUNT STRUCTURE
```rust
#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,         // Mint authority
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,  // Mint account
    #[account(mut)]
    pub token_account: InterfaceAccount<'info, TokenAccount>,  // Destination
    pub token_program: Interface<'info, TokenInterface>,       // Token program
}
```

### MINTING LOGIC
```rust
pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
    // Create accounts struct for the CPI
    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
    };
    
    // Get token program account
    let cpi_program = ctx.accounts.token_program.to_account_info();
    
    // Create CPI context
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
    
    // Make the CPI to mint tokens
    token_interface::mint_to(cpi_context, amount)?;
    Ok(())
}
```

## ADVANCED PATTERN: PDA AS MINT AUTHORITY

### MINT SETUP WITH PDA AUTHORITY
```rust
#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 6,
        mint::authority = mint,       // PDA as mint authority
        mint::freeze_authority = mint,
        seeds = [b"mint"],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```

### MINTING WITH PDA SIGNING
```rust
pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
    // Create seeds array for PDA signing
    let signer_seeds: &[&[&[u8]]] = &[&[b"mint", &[ctx.bumps.mint]]];

    // Create accounts struct for the CPI
    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.mint.to_account_info(),  // PDA as authority
    };
    
    // Get token program account
    let cpi_program = ctx.accounts.token_program.to_account_info();
    
    // Create CPI context with PDA signer
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts)
        .with_signer(signer_seeds);
    
    // Make the CPI to mint tokens
    token_interface::mint_to(cpi_context, amount)?;
    Ok(())
}
```

## ACCOUNT VALIDATION FOR PDA MINT
```rust
#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"mint"],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
```

## CLIENT IMPLEMENTATION
```typescript
// Find PDA for mint
const [mint, mintBump] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("mint")],
  program.programId,
);

// Mint tokens (amount in base units: 100 = 0.0001 if decimals = 6)
await program.methods
  .mintTokens(new anchor.BN(100))
  .accounts({
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  })
  .rpc();

// Get ATA address to check balance
const associatedTokenAccount = await getAssociatedTokenAddress(
  mint,
  program.provider.publicKey,
  false,
  TOKEN_2022_PROGRAM_ID,
);

// Get token account data
const tokenAccount = await getAccount(
  program.provider.connection,
  associatedTokenAccount,
  "confirmed",
  TOKEN_2022_PROGRAM_ID,
);
```

## KEY CONCEPTS
- AMOUNT: Token amount in base units (adjusted by decimals)
- PDA SIGNING: Use `.with_signer(signer_seeds)` with PDA mint authority
- DUAL PURPOSE: Same PDA can be both mint account address and mint authority
>>> prompts/structured-anchor-docs/solpg.mdx
# SOLANA PLAYGROUND (SOLPG)

## OVERVIEW
- Browser-based Solana development environment
- Build, deploy, and test Solana programs without local setup
- URL: https://beta.solpg.io

## SETUP PROCESS

### WALLET CREATION
1. Click "Not connected" button at bottom left
2. Save your wallet keypair (optional)
3. Connect to the Playground
4. Wallet is stored in browser local storage

### GETTING DEVNET SOL
- Run in terminal: `solana airdrop 5`
- Or use Web Faucet: https://faucet.solana.com/
- SOL is needed for:
  - Creating accounts for data/program storage
  - Paying transaction fees

## DEVELOPMENT WORKFLOW

### CREATE PROJECT
1. Click "Create a new project"
2. Enter project name
3. Select "Anchor" framework
4. Default program includes:
   - `initialize` instruction that stores data in an account
   - Account structure with a u64 data field

### BUILD PROGRAM
1. Run `build` in terminal or click "Build" button
2. Program ID in `declare_id!()` gets updated automatically

### DEPLOY PROGRAM
1. Run `deploy` in terminal or click "Deploy" button
2. Deploys to devnet by default
3. SOL is allocated to store program on-chain

### TEST PROGRAM
1. Default test file: `tests/anchor.test.ts`
2. Run `test` in terminal or click "Test" button
3. Test creates a new account and stores data in it
4. Verifies stored data matches expected value

### VIEWING TRANSACTION LOGS
1. Run `solana confirm -v [TRANSACTION_HASH]`
2. View on explorer: https://explorer.solana.com/?cluster=devnet

### CLOSE PROGRAM (OPTIONAL)
1. Run `solana program close [PROGRAM_ID]`
2. Recovers SOL allocated to program storage

## KEY CODE COMPONENTS

### PROGRAM CODE
```rust
// Program ID updated on build
declare_id!("11111111111111111111111111111111");

// Program instruction
#[program]
mod hello_anchor {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, data: u64) -> Result<()> {
        ctx.accounts.new_account.data = data;
        msg!("Changed data to: {}!", data);
        Ok(())
    }
}

// Account validation
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = signer, space = 8 + 8)]
    pub new_account: Account<'info, NewAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Account data structure
#[account]
pub struct NewAccount {
    data: u64
}
```

### TEST CODE
```typescript
it("initialize", async () => {
  // Generate account keypair
  const newAccountKp = new web3.Keypair();
  
  // Send transaction
  const data = new BN(42);
  const txHash = await pg.program.methods
    .initialize(data)
    .accounts({
      newAccount: newAccountKp.publicKey,
      signer: pg.wallet.publicKey,
      systemProgram: web3.SystemProgram.programId,
    })
    .signers([newAccountKp])
    .rpc();
    
  // Confirm transaction
  await pg.connection.confirmTransaction(txHash);
  
  // Fetch and verify account data
  const newAccount = await pg.program.account.newAccount.fetch(
    newAccountKp.publicKey,
  );
  assert(data.eq(newAccount.data));
})
```
>>> prompts/structured-anchor-docs/idl.mdx
# ANCHOR IDL (INTERFACE DESCRIPTION LANGUAGE)

## OVERVIEW
- IDL: JSON file describing program's instructions and accounts
- PURPOSE: Standardizes program-client interactions, enables client code generation
- LOCATION: Generated at `/target/idl/<program-name>.json` by `anchor build`

## IDL COMPONENTS

### INSTRUCTIONS
- Corresponds to program instructions defined with `#[program]`
- CONTAINS:
  - Name (e.g., "initialize")
  - Discriminator (8-byte unique identifier)
  - Accounts (required for the instruction)
  - Args (parameters for the instruction)

### ACCOUNTS
- Corresponds to structs annotated with `#[account]`
- CONTAINS:
  - Name (e.g., "NewAccount")
  - Discriminator (8-byte unique identifier)
  - Field definitions in the "types" section

### TYPES
- Defines structs used by the program
- CONTAINS:
  - Field names and types for each account struct

## DISCRIMINATORS

### INSTRUCTION DISCRIMINATORS
- First 8 bytes of SHA256 hash of `global:<instruction_name>`
- Example: `sha256("global:initialize")`
- Used to identify which instruction to execute
- Included as first 8 bytes of instruction data

### ACCOUNT DISCRIMINATORS
- First 8 bytes of SHA256 hash of `account:<account_name>`
- Example: `sha256("account:NewAccount")`
- Used to identify account type during deserialization
- Stored as first 8 bytes in the account data

## CLIENT USAGE
- TypeScript clients can be generated from IDL
- PATTERN:
  ```ts
  // Program initialization
  const program = anchor.workspace.ProgramName as Program<ProgramType>;
  
  // Instruction invocation
  await program.methods
    .instructionName(params)
    .accounts({ accountName: pubkey })
    .signers([keypair])
    .rpc();
  
  // Account data fetching
  const accountData = await program.account.accountName.fetch(pubkey);
  ```
>>> prompts/structured-anchor-docs/errors.mdx
# CUSTOM ERRORS IN ANCHOR

## RESULT TYPE
- All Anchor instruction handlers return `Result<T>` type
- Type alias for standard Rust `Result<T, E>` where E is Anchor's `Error` type
- Syntax: `pub type Result<T> = std::result::Result<T, error::Error>;`

## ERROR TYPES

### ANCHOR ERROR
```rust
pub enum Error {
    AnchorError(Box<AnchorError>),
    ProgramError(Box<ProgramErrorWithOrigin>),
}
```

#### ProgramErrorWithOrigin
- Wraps standard Solana `ProgramError`
- Provides error origin tracking

#### AnchorError
- Structure:
  ```rust
  pub struct AnchorError {
      pub error_name: String,
      pub error_code_number: u32,
      pub error_msg: String,
      pub error_origin: Option<ErrorOrigin>,
      pub compared_values: Option<ComparedValues>,
  }
  ```
- Categories:
  1. Internal Anchor Errors (built-in)
  2. Custom Program Errors (user-defined)

### ERROR CODE RANGES
| Error Code | Description                           |
|------------|---------------------------------------|
| >= 100     | Instruction error codes               |
| >= 1000    | IDL error codes                       |
| >= 2000    | Constraint error codes                |
| >= 3000    | Account error codes                   |
| >= 4100    | Misc error codes                      |
| = 5000     | Deprecated error code                 |
| >= 6000    | Starting point for custom user errors |

## DEFINING CUSTOM ERRORS

### #[ERROR_CODE] ATTRIBUTE
```rust
#[error_code]
pub enum MyError {
    #[msg("My custom error message")]
    MyCustomError,
    #[msg("My second custom error message")]
    MySecondCustomError,
}
```

## ERROR HANDLING MACROS

### ERR!
- Returns a custom error from program
- Syntax: `err!(MyError::CustomErrorVariant)`

### REQUIRE!
- Combines condition check with error return
- Syntax: `require!(condition, ErrorType::ErrorVariant)`
- Example:
  ```rust
  require!(data.amount < 100, MyError::AmountTooLarge);
  ```

### OTHER REQUIRE MACROS
| Macro               | Description                                        |
|---------------------|----------------------------------------------------|
| `require!`          | Condition must be true                             |
| `require_eq!`       | Two non-pubkey values must be equal                |
| `require_neq!`      | Two non-pubkey values must not be equal            |
| `require_keys_eq!`  | Two pubkeys must be equal                          |
| `require_keys_neq!` | Two pubkeys must not be equal                      |
| `require_gt!`       | First value must be greater than second            |
| `require_gte!`      | First value must be greater than or equal to second|

## CLIENT-SIDE ERROR HANDLING
```typescript
try {
  await program.methods.someMethod().rpc();
} catch (error) {
  // Access error details
  console.log(error.error.errorCode.code);    // Error enum variant name
  console.log(error.error.errorCode.number);  // Error code number
  console.log(error.error.errorMessage);      // Error message
  console.log(error.error.origin);            // File and line info
}
```
>>> prompts/structured-anchor-docs/extensions.mdx
# TOKEN EXTENSIONS (TOKEN-2022)

## OVERVIEW
- DEFINITION: Optional functionality added to token mints or accounts in the Token Extensions Program (Token 2022)
- TIMING: Must be enabled during account creation; cannot be added later
- STORAGE: Extension data stored in `tlv_data` field beyond base account structure
- COMPATIBILITY: Some extensions are mutually incompatible (e.g., NonTransferable + TransferFeeConfig)

## AVAILABLE EXTENSIONS

### MINT EXTENSIONS
- `TransferFeeConfig`: Adds transfer fee rate and management authorities
- `MintCloseAuthority`: Enables mint account closure
- `DefaultAccountState`: Sets default state for new token accounts
- `NonTransferable`: Prevents token transfers
- `InterestBearingConfig`: Tokens accrue interest over time
- `PermanentDelegate`: Configures a permanent authority
- `TransferHook`: Requires CPI to a program implementing transfer hook interface
- `MetadataPointer`: Points to account holding metadata
- `TokenMetadata`: Stores token metadata directly on mint
- `GroupPointer`: Points to account with group configurations
- `TokenGroup`: Stores token group configurations
- `ConfidentialMintBurn`: Enables confidential token minting and burning
- `ScaledUiAmount`: Scales token UI amount display
- `Pausable`: Allows pausing mint/burn/transfer operations

### TOKEN ACCOUNT EXTENSIONS
- `TransferFeeAmount`: Tracks withheld transfer fees
- `ConfidentialTransferAccount`: Stores state for confidential transfers
- `ImmutableOwner`: Prevents changing account owner
- `MemoTransfer`: Requires memo for inbound transfers
- `CpiGuard`: Blocks privileged operations via CPI
- `NonTransferableAccount`: Indicates tokens from non-transferable mint
- `TransferHookAccount`: Indicates tokens from mint with transfer hook
- `PausableAccount`: Indicates account belongs to pausable mint

## EXTENSION TYPE ENUM
```rust
#[repr(u16)]
pub enum ExtensionType {
    Uninitialized,
    TransferFeeConfig,
    TransferFeeAmount,
    MintCloseAuthority,
    ConfidentialTransferMint,
    ConfidentialTransferAccount,
    DefaultAccountState,
    ImmutableOwner,
    MemoTransfer,
    NonTransferable,
    InterestBearingConfig,
    CpiGuard,
    PermanentDelegate,
    NonTransferableAccount,
    TransferHook,
    TransferHookAccount,
    ConfidentialTransferFeeConfig,
    ConfidentialTransferFeeAmount,
    MetadataPointer,
    TokenMetadata,
    GroupPointer,
    TokenGroup,
    GroupMemberPointer,
    TokenGroupMember,
    ConfidentialMintBurn,
    ScaledUiAmount,
    Pausable,
    PausableAccount,
    // Test extensions omitted
}
```

## IMPLEMENTATION IN ANCHOR

### REQUIRED IMPORTS
```rust
use anchor_spl::token_2022_extensions::{
    // Import specific extension modules as needed
    transfer_fee,
    metadata_pointer,
    // etc.
};
```

### EXTENSION DATA STRUCTURE
- Data structure with extension data:
```rust
#[derive(Debug, PartialEq)]
pub struct PodStateWithExtensions<'data, S: BaseState + Pod> {
    pub base: &'data S,         // Base mint or account data
    tlv_data: &'data [u8],      // Extension data (TLV format)
}
```

## USAGE NOTES

- INITIALIZATION: All extensions must be enabled during account creation
- PLANNING: Carefully plan which extensions to enable as they cannot be added later
- COMPATIBILITY: Check for extension conflicts before implementation
- ANCHOR SUPPORT: Not all extensions have Anchor helper functions; manual CPI may be needed

## RESOURCES
- [Token 2022 Examples Repository](https://github.com/solana-developers/program-examples/tree/main/tokens/token-2022)
- [Token Extensions Source Code](https://github.com/solana-program/token-2022/tree/main/program/src/extension)
- [Anchor SPL Token 2022 Extensions](https://github.com/coral-xyz/anchor/tree/0e5285aecdf410fa0779b7cd09a47f235882c156/spl/src/token_2022_extensions)
>>> prompts/structured-anchor-docs/pda.mdx
# PROGRAM DERIVED ADDRESS (PDA) IN ANCHOR

## OVERVIEW
- Deterministic addresses derived from seeds and program ID
- Used for creating predictable account addresses
- Enable programs to sign for accounts without private keys

## PDA CONSTRAINTS
- `seeds`: Array of values used for derivation (required)
- `bump`: Bump seed value to ensure address is off-curve (required)
- `seeds::program`: Optional program ID if deriving from different program
- Note: `seeds` and `bump` must be used together

## SEED PATTERNS

### NO OPTIONAL SEEDS
```rust
#[account(
    seeds = [],
    bump,
)]
pub pda_account: SystemAccount<'info>,
```

### STATIC SEED
```rust
#[account(
    seeds = [b"hello_world"],
    bump,
)]
pub pda_account: SystemAccount<'info>,
```

### MULTIPLE SEEDS WITH REFERENCES
```rust
#[account(
    seeds = [b"hello_world", signer.key().as_ref()],
    bump,
)]
pub pda_account: SystemAccount<'info>,
```

## BUMP HANDLING

### AUTOMATIC CALCULATION
```rust
#[account(
    seeds = [b"hello_world"],
    bump,
)]
pub pda_account: SystemAccount<'info>,
```

### EXPLICIT BUMP VALUE
```rust
#[account(
    seeds = [b"hello_world"],
    bump = pda_account.bump_seed,
)]
pub pda_account: Account<'info, CustomAccount>,
```

## CROSS-PROGRAM PDA
```rust
#[account(
    seeds = [b"hello_world"],
    bump,
    seeds::program = other_program.key(),
)]
pub pda_account: SystemAccount<'info>,
```

## CREATING PDA ACCOUNTS
```rust
#[account(
    init,
    payer = signer,
    space = 8 + 1,
    seeds = [b"hello_world", signer.key().as_ref()],
    bump,
)]
pub pda_account: Account<'info, CustomAccount>,
```

## IDL INTEGRATION
- PDA seeds defined in constraints are included in program's IDL
- Static seeds converted to byte values
- Dynamic seeds included as references to accounts
- Anchor client automatically resolves PDA addresses
- Simplifies client-side instruction building
>>> prompts/structured-anchor-docs/cpi.mdx
# CROSS PROGRAM INVOCATION (CPI)

## OVERVIEW
- CPIs allow one program to invoke instructions of another program
- Enables composability between different Solana programs
- Requires: program ID, accounts, and instruction data

## IMPLEMENTATION APPROACHES

### APPROACH 1: ANCHOR HELPER FUNCTIONS
```rust
// Import Anchor helper 
use anchor_lang::system_program::{transfer, Transfer};

// CPI Implementation
let cpi_context = CpiContext::new(
    program_id,
    Transfer {
        from: from_pubkey,
        to: to_pubkey,
    },
);
transfer(cpi_context, amount)?;
```

### APPROACH 2: INVOKE + INSTRUCTION BUILDER
```rust
// Import solana_program tools
use anchor_lang::solana_program::{program::invoke, system_instruction};

// CPI Implementation
let instruction = &system_instruction::transfer(
    &from_pubkey.key(), 
    &to_pubkey.key(), 
    amount
);
invoke(instruction, &[from_pubkey, to_pubkey, program_id])?;
```

### APPROACH 3: MANUAL INSTRUCTION CONSTRUCTION
```rust
// Manual instruction construction
let account_metas = vec![
    AccountMeta::new(from_pubkey.key(), true),
    AccountMeta::new(to_pubkey.key(), false),
];

// Create instruction data buffer
let instruction_discriminator: u32 = 2;
let mut instruction_data = Vec::with_capacity(4 + 8);
instruction_data.extend_from_slice(&instruction_discriminator.to_le_bytes());
instruction_data.extend_from_slice(&amount.to_le_bytes());

// Create and invoke instruction
let instruction = Instruction {
    program_id: program_id.key(),
    accounts: account_metas,
    data: instruction_data,
};
invoke(&instruction, &[from_pubkey, to_pubkey, program_id])?;
```

## CPI WITH PDA SIGNING

### USING CPICONTEXT
```rust
// Get seeds and bump
let seed = to_pubkey.key();
let bump_seed = ctx.bumps.pda_account;
let signer_seeds: &[&[&[u8]]] = &[&[b"pda", seed.as_ref(), &[bump_seed]]];

// Create CPI context with signer
let cpi_context = CpiContext::new(
    program_id,
    Transfer {
        from: from_pubkey,
        to: to_pubkey,
    },
)
.with_signer(signer_seeds);

transfer(cpi_context, amount)?;
```

### USING INVOKE_SIGNED
```rust
// Get seeds and bump
let seed = to_pubkey.key();
let bump_seed = ctx.bumps.pda_account;
let signer_seeds: &[&[&[u8]]] = &[&[b"pda", seed.as_ref(), &[bump_seed]]];

// Create instruction
let instruction = &system_instruction::transfer(
    &from_pubkey.key(), 
    &to_pubkey.key(), 
    amount
);

// Invoke with PDA signing
invoke_signed(
    instruction,
    &[from_pubkey, to_pubkey, program_id],
    signer_seeds
)?;
```

## ACCOUNTS DEFINITION
```rust
#[derive(Accounts)]
pub struct SolTransfer<'info> {
    #[account(mut)]
    sender: Signer<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}
```

## PDA ACCOUNTS DEFINITION
```rust
#[derive(Accounts)]
pub struct SolTransfer<'info> {
    #[account(
        mut,
        seeds = [b"pda", recipient.key().as_ref()],
        bump,
    )]
    pda_account: SystemAccount<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}
>>> prompts/structured-anchor-docs/anchor-toml.mdx
# ANCHOR.TOML CONFIGURATION

## PROVIDER (REQUIRED)
- Defines wallet and cluster used for all commands
```toml
[provider]
cluster = "localnet"                # Cluster: localnet, devnet, mainnet-beta
wallet = "~/.config/solana/id.json" # Keypair used for all commands
```

## SCRIPTS (REQUIRED FOR TESTING)
- Scripts executed via `anchor run <script>`
- `test` script is executed by `anchor test`
```toml
[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
```

## FEATURES
- `resolution`: Enables IDL account resolution (default: true)
```toml
[features]
resolution = true
```

## WORKSPACE
- `types`: Directory where IDL TypeScript definitions are copied after build
  ```toml
  [workspace]
  types = "app/src/idl/"
  ```

- `members`: Paths to programs in workspace (relative to Anchor.toml)
  ```toml
  [workspace]
  members = [
      "programs/*",
      "other_place/my_program"
  ]
  ```

- `exclude`: Programs to exclude from workspace
  ```toml
  [workspace]
  exclude = [
      "programs/my_program"
  ]
  ```

## PROGRAMS
- Defines program addresses for different networks
```toml
[programs.localnet]
my_program = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

[programs.devnet]
my_program = "Different_Address_For_Devnet"
```

## TEST
- `startup_wait`: Milliseconds to wait for validator startup
  ```toml
  [test]
  startup_wait = 10000
  ```

- `genesis`: Programs to load at genesis when starting test validator
  ```toml
  [[test.genesis]]
  address = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"
  program = "dex.so"
  
  [[test.genesis]]
  address = "22Y43yTVxuUkoRKdm9thyRhQ3SdgQS7c7kB6UNCiaczD"
  program = "swap.so"
  upgradeable = true
  ```

- `upgradeable`: Deploy program as upgradeable for testing upgrade authority
  ```toml
  [test]
  upgradeable = true
  ```

## TEST.VALIDATOR
- Options passed to `solana-test-validator`
```toml
[test.validator]
url = "https://api.mainnet-beta.solana.com" # URL for account cloning
warp_slot = 1337                           # Warp ledger to this slot
slots_per_epoch = 5                        # Override slots per epoch
rpc_port = 1337                            # JSON RPC port
limit_ledger_size = 1337                   # Max shreds in root slots
ledger = "test-ledger"                     # Ledger location
gossip_port = 1337                         # Gossip port
gossip_host = "127.0.0.1"                  # Gossip host
faucet_sol = 1337                          # SOL for faucet
faucet_port = 1337                         # Faucet port
dynamic_port_range = "1337 - 13337"        # Port range
bind_address = "0.0.0.0"                   # Bind IP
```

### VALIDATOR CLONING
- Clone accounts from another cluster for testing
```toml
[test.validator]
url = "https://api.mainnet-beta.solana.com"

[[test.validator.clone]]
address = "7NL2qWArf2BbEBBH1vTRZCsoNqFATTddH6h8GkVvrLpG"

# BPF upgradeable programs clone both program and data account
[[test.validator.clone]]
address = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
```

### VALIDATOR ACCOUNTS
- Upload account data from JSON files
```toml
[[test.validator.account]]
address = "Ev8WSPQsGb4wfjybqff5eZNcS3n6HaMsBkMk9suAiuM"
filename = "some_account.json"
```

## TOOLCHAIN
- Override toolchain versions
```toml
[toolchain]
anchor_version = "0.31.1"   # Requires avm installed
solana_version = "2.1.21"   # Applies to all Solana tools
package_manager = "yarn"    # JS package manager to use
```
>>> prompts/structured-anchor-docs/create-token-account.mdx
# CREATING TOKEN ACCOUNTS IN ANCHOR

## TOKEN ACCOUNT OVERVIEW
- DEFINITION: Account that stores a user's token balance for a specific token mint
- STRUCTURE:
  ```rust
  pub struct Account {
    pub mint: Pubkey,              // Token mint address
    pub owner: Pubkey,             // Authority who can transfer tokens
    pub amount: u64,               // Token balance
    pub delegate: COption<Pubkey>, // Optional delegate authority
    pub state: AccountState,       // Account state (frozen/unfrozen)
    pub is_native: COption<u64>,   // For wrapped SOL
    pub delegated_amount: u64,     // Amount delegated
    pub close_authority: COption<Pubkey> // Can close account
  }
  ```
- OWNERSHIP: Token accounts are owned by the Token Program/Token Extension Program

## ASSOCIATED TOKEN ACCOUNTS (ATAs)

- DEFINITION: Token accounts with deterministically derived addresses
- CREATED BY: Associated Token Program
- ADDRESS DERIVATION:
  ```rust
  // Derived from:
  [
    wallet_address.to_bytes(),    // Owner's public key
    token_program_id.to_bytes(),  // Token Program ID
    token_mint_address.to_bytes() // Token mint address
  ]
  ```
- PURPOSE: Standardized way to find a user's token account for any mint

## IMPLEMENTATION IN ANCHOR

### REQUIRED IMPORTS
```rust
// For ATAs
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

// For PDA token accounts
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
```

### ACCOUNT TYPES
- `InterfaceAccount<'info, TokenAccount>`: Token account wrapper compatible with both token programs
- `Interface<'info, TokenInterface>`: Program interface for token interactions
- `Program<'info, AssociatedToken>`: Reference to Associated Token Program

### CONSTRAINT SETS

#### ASSOCIATED TOKEN ACCOUNT CONSTRAINTS
- `init`/`init_if_needed`: Creates account (or reuses existing with init_if_needed)
- `payer`: Specifies rent-paying account
- `associated_token::mint`: Sets token mint
- `associated_token::authority`: Sets token owner
- `associated_token::token_program`: Specifies token program implementation

#### PDA TOKEN ACCOUNT CONSTRAINTS
- `init`/`init_if_needed`: Creates account (or reuses existing with init_if_needed)
- `payer`: Specifies rent-paying account
- `token::mint`: Sets token mint
- `token::authority`: Sets token owner
- `token::token_program`: Specifies token program implementation
- `seeds` + `bump`: For PDA derivation

## CREATION APPROACHES

### 1. CREATING ASSOCIATED TOKEN ACCOUNTS (ATAs)
- USAGE: Standard approach for user token accounts
- IMPLEMENTATION:
  ```rust
  #[derive(Accounts)]
  pub struct CreateTokenAccount<'info> {
      #[account(mut)]
      pub signer: Signer<'info>,
      #[account(
          init_if_needed,
          payer = signer,
          associated_token::mint = mint,
          associated_token::authority = signer,
          associated_token::token_program = token_program,
      )]
      pub token_account: InterfaceAccount<'info, TokenAccount>,
      pub mint: InterfaceAccount<'info, Mint>,
      pub token_program: Interface<'info, TokenInterface>,
      pub associated_token_program: Program<'info, AssociatedToken>,
      pub system_program: Program<'info, System>,
  }
  ```
- CLIENT USAGE:
  ```typescript
  // Derive ATA address
  const associatedTokenAccount = await getAssociatedTokenAddress(
    mint,
    program.provider.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID
  );
  
  // Create ATA
  await program.methods
    .createTokenAccount()
    .accounts({
      mint: mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .rpc();
  ```

### 2. CREATING PDA TOKEN ACCOUNTS
- USAGE: Program-controlled token accounts with deterministic addresses
- IMPLEMENTATION:
  ```rust
  #[derive(Accounts)]
  pub struct CreateTokenAccount<'info> {
      #[account(mut)]
      pub signer: Signer<'info>,
      #[account(
          init_if_needed,
          payer = signer,
          token::mint = mint,
          token::authority = token_account, // Self-authority pattern
          token::token_program = token_program,
          seeds = [b"token"],
          bump
      )]
      pub token_account: InterfaceAccount<'info, TokenAccount>,
      pub mint: InterfaceAccount<'info, Mint>,
      pub token_program: Interface<'info, TokenInterface>,
      pub system_program: Program<'info, System>,
  }
  ```
- CLIENT USAGE:
  ```typescript
  // Derive token PDA address
  const [token, tokenBump] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("token")],
    program.programId
  );
  
  // Create token account
  await program.methods
    .createTokenAccount()
    .accounts({
      mint: mint,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .rpc();
  ```

## ENABLING INIT-IF-NEEDED
- Add feature flag in Cargo.toml:
  ```toml
  [dependencies]
  anchor-lang = { version = "0.31.1", features = ["init-if-needed"] }
  ```

## ADVANCED PATTERNS
- SELF-AUTHORITY: Set token authority to the token account's own address
- PDA AUTHORITY: Use a PDA as token authority to enable program-controlled transfers
- COMBINED APPROACH: Same PDA can be both token account address and authority

## DEFAULT CODE SAMPLES

### PROGRAM CODE (lib.rs)
```rust
use anchor_lang::prelude::*;

declare_id!("3ynNB373Q3VAzKp7m4x238po36hjAGFXFJB4ybN2iTyg");

#[program]
pub mod my_project {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
```

### TYPESCRIPT TEST
```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MyProject } from "../target/types/my_project";

describe("my-project", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.MyProject as Program<MyProject>;

  it("Is initialized!", async () => {
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
```
>>> prompts/structured-anchor-docs/declare-program.mdx
# DECLARE_PROGRAM MACRO

## OVERVIEW
- PURPOSE: Enables dependency-free program composability
- FUNCTION: Generates Rust modules from a program's IDL for both on-chain and off-chain code
- LOCATION: IDL file must be placed in a `/idls` directory (e.g., `/idls/program_name.json`)

## GENERATED MODULES

- `cpi`: Helper functions for making cross-program invocations
- `client`: Accounts and arguments for building program instructions
- `account`: Account data types defined in the program
- `program`: Program ID constant
- `constants`: Program constants
- `events`: Program event definitions
- `types`: Program type definitions
- `errors`: Program error definitions

## USAGE PATTERNS

### ON-CHAIN CROSS-PROGRAM INVOCATION (CPI)

1. SETUP:
   ```rust
   declare_program!(example);  // Looks for /idls/example.json
   use example::{
       accounts::Counter,
       cpi::{
           self,
           accounts::{Increment, Initialize},
       },
       program::Example,
   };
   ```

2. ACCOUNT VALIDATION:
   ```rust
   #[derive(Accounts)]
   pub struct IncrementCpi<'info> {
       #[account(mut)]
       pub counter: Account<'info, Counter>,
       pub example_program: Program<'info, Example>,
   }
   ```

3. INSTRUCTION INVOCATION:
   ```rust
   pub fn increment_cpi(ctx: Context<IncrementCpi>) -> Result<()> {
       let cpi_ctx = CpiContext::new(
           ctx.accounts.example_program.to_account_info(),
           Increment {
               counter: ctx.accounts.counter.to_account_info(),
           },
       );
       cpi::increment(cpi_ctx)?;
       Ok(())
   }
   ```

### OFF-CHAIN CLIENT USAGE

1. SETUP:
   ```rust
   declare_program!(example);
   use example::{accounts::Counter, client::accounts, client::args};
   ```

2. PROGRAM CONNECTION:
   ```rust
   let program = provider.program(example::ID)?;
   ```

3. INSTRUCTION BUILDING:
   ```rust
   let initialize_ix = program
       .request()
       .accounts(accounts::Initialize {
           counter: counter.pubkey(),
           payer: program.payer(),
           system_program: system_program::ID,
       })
       .args(args::Initialize)
       .instructions()?
       .remove(0);
   ```

4. TRANSACTION EXECUTION:
   ```rust
   let signature = program
       .request()
       .instruction(initialize_ix)
       .instruction(increment_ix)
       .signer(&counter)
       .send()
       .await?;
   ```

5. ACCOUNT DATA FETCHING:
   ```rust
   let counter_account: Counter = program.account::<Counter>(counter.pubkey()).await?;
   ```

## KEY BENEFITS

- CONSISTENCY: Same code can be used for both on-chain and off-chain interactions
- TYPED INTERFACES: Type-safe interaction with program instructions and accounts
- NO DEPENDENCIES: Programs can interact without importing each other as dependencies
- SIMPLIFIED WORKFLOW: Generated from IDL without requiring program source code
>>> prompts/structured-anchor-docs/zero-copy.mdx
# ZERO COPY DESERIALIZATION

## OVERVIEW
- Allows direct memory access to account data without copying
- Optimized for large account data structures
- Uses C-compatible memory layout (repr(C))
- Requires bytemuck crate for Pod and Zeroable traits

## SETUP

### DEPENDENCIES
```toml
[dependencies]
bytemuck = { version = "1.20.0", features = ["min_const_generics"] }
anchor-lang = "0.31.1"
```

### DEFINING ZERO COPY ACCOUNTS
```rust
#[account(zero_copy)]
pub struct Data {
    pub data: [u8; 10232],  // 10240 bytes - 8 bytes account discriminator
}
```

- Automatically implements:
  - `#[derive(Copy, Clone)]`
  - `#[derive(bytemuck::Zeroable)]`
  - `#[derive(bytemuck::Pod)]`
  - `#[repr(C)]`

## USAGE PATTERNS

### ACCOUNT LOADING
- Use `AccountLoader<'info, T>` to handle zero-copy accounts
```rust
#[derive(Accounts)]
pub struct InstructionAccounts<'info> {
    pub zero_copy_account: AccountLoader<'info, Data>,
}
```

### LOADING METHODS
1. `load_init()` - Initialize new account and set discriminator
2. `load_mut()` - Mutable access for updates
3. `load()` - Read-only access

### INITIALIZATION APPROACHES

#### STANDARD INIT (UP TO 10240 BYTES)
```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        space = 8 + 10232,  // 8 bytes for discriminator
        payer = payer,
    )]
    pub data_account: AccountLoader<'info, Data>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let account = &mut ctx.accounts.data_account.load_init()?;
    account.data = [1; 10232];
    Ok(())
}
```

#### LARGE ACCOUNT INIT (UP TO 10MB)
```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]  // Verify account not initialized
    pub data_account: AccountLoader<'info, Data>,
}

pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let account = &mut ctx.accounts.data_account.load_init()?;
    account.data = [1; 10_485_752];  // 10MB - 8 bytes
    Ok(())
}
```

- Requires separate SystemProgram createAccount call in client:
```typescript
// Create account with SystemProgram
const createAccountInstruction = anchor.web3.SystemProgram.createAccount({
  fromPubkey: program.provider.publicKey,
  newAccountPubkey: dataAccount.publicKey,
  space: 10_485_760,  // 10MB max account size
  lamports: await connection.getMinimumBalanceForRentExemption(10_485_760),
  programId: program.programId,
});

// Initialize account data in separate instruction
const initializeInstruction = await program.methods
  .initialize()
  .accounts({ dataAccount: dataAccount.publicKey })
  .instruction();

// Send transaction with both instructions
const transaction = new anchor.web3.Transaction()
  .add(createAccountInstruction, initializeInstruction);
```

### UPDATING ACCOUNTS
```rust
#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    pub data_account: AccountLoader<'info, Data>,
}

pub fn update(ctx: Context<Update>) -> Result<()> {
    let account = &mut ctx.accounts.data_account.load_mut()?;
    account.data = [2; 10232];
    Ok(())
}
```

### READING ACCOUNTS
```rust
#[derive(Accounts)]
pub struct ReadOnly<'info> {
    pub data_account: AccountLoader<'info, Data>,
}

pub fn read_only(ctx: Context<ReadOnly>) -> Result<()> {
    let account = &ctx.accounts.data_account.load()?;
    msg!("First 10 bytes: {:?}", &account.data[..10]);
    Ok(())
}
```

## LIMITATIONS
- `init` constraint limited to 10240 bytes (CPI restriction)
- Zero-copy types must satisfy Pod and Zeroable traits:
  - No references
  - No Drop implementations
  - All fields must be Pod and Zeroable
  - Must use primitive types or arrays/structs of primitives
- Maximum account size is 10MB (10,485,760 bytes)
>>> prompts/structured-anchor-docs/account-types.mdx
# ANCHOR ACCOUNT TYPES

## ACCOUNT<'INFO, T>
- **Purpose**: Main account container with ownership verification
- **Validation**: Checks account ownership and deserializes data
- **Usage**:
  ```rust
  pub account: Account<'info, CustomAccountType>
  ```

## ACCOUNTINFO<'INFO>
- **Purpose**: Raw account data access without validation
- **Note**: Prefer UncheckedAccount for clarity
- **Usage**:
  ```rust
  /// CHECK: AccountInfo is unchecked
  pub unchecked_account: AccountInfo<'info>
  ```

## ACCOUNTLOADER<'INFO, T>
- **Purpose**: On-demand zero-copy deserialization
- **Used with**: #[account(zero_copy)] structs
- **Usage**:
  ```rust
  pub account: AccountLoader<'info, ZeroCopyAccountType>
  ```

## BOX<ACCOUNT<'INFO, T>>
- **Purpose**: Reduce stack usage for large accounts
- **Usage**:
  ```rust
  pub account: Box<Account<'info, AccountType>>
  ```

## INTERFACE<'INFO, T>
- **Purpose**: Validates account is one of multiple programs
- **Example**: Token or Token2022 program interface
- **Usage**:
  ```rust
  pub program: Interface<'info, TokenInterface>
  ```

## INTERFACEACCOUNT<'INFO, T>
- **Purpose**: Account compatible with multiple program implementations
- **Example**: Token or Token2022 mint/token accounts
- **Usage**:
  ```rust
  pub mint: InterfaceAccount<'info, Mint>
  pub token: InterfaceAccount<'info, TokenAccount>
  ```

## OPTION<ACCOUNT<'INFO, T>>
- **Purpose**: Optional account parameters
- **Usage**:
  ```rust
  pub account: Option<Account<'info, AccountType>>
  ```

## PROGRAM<'INFO, T>
- **Purpose**: Validates account is specific program
- **Usage**:
  ```rust
  pub system_program: Program<'info, System>
  pub token_program: Program<'info, Token>
  ```

## SIGNER<'INFO>
- **Purpose**: Validates account signed transaction
- **Usage**:
  ```rust
  pub signer: Signer<'info>
  ```

## SYSTEMACCOUNT<'INFO>
- **Purpose**: Validates account owned by System Program
- **Usage**:
  ```rust
  pub account: SystemAccount<'info>
  ```

## SYSVAR<'INFO, T>
- **Purpose**: Validates and deserializes sysvar accounts
- **Usage**:
  ```rust
  pub rent: Sysvar<'info, Rent>
  pub clock: Sysvar<'info, Clock>
  ```

## UNCHECKEDACCOUNT<'INFO>
- **Purpose**: Explicit wrapper for accounts without validation
- **Usage**:
  ```rust
  /// CHECK: No checks performed
  pub account: UncheckedAccount<'info>
  ```
>>> prompts/structured-anchor-docs/create-mint.mdx
# CREATING TOKEN MINTS IN ANCHOR

## MINT ACCOUNT OVERVIEW
- DEFINITION: Account representing a token on Solana, storing global token metadata
- STRUCTURE:
  ```rust
  pub struct Mint {
    pub mint_authority: COption<Pubkey>,  // Can mint new tokens
    pub supply: u64,                      // Total supply
    pub decimals: u8,                     // Decimal precision
    pub is_initialized: bool,             // Initialization status
    pub freeze_authority: COption<Pubkey> // Can freeze token accounts
  }
  ```
- IDENTIFIERS: Mint address uniquely identifies a token (e.g., USDC: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
- COMPATIBILITY: Same structure in both Token Program and Token Extension Program

## IMPLEMENTATION IN ANCHOR

### REQUIRED IMPORTS
```rust
use anchor_spl::token_interface::{Mint, TokenInterface};
```

### ACCOUNT TYPES
- `InterfaceAccount<'info, Mint>`: Wrapper compatible with both token programs
- `Interface<'info, TokenInterface>`: Program interface for token interactions

### ACCOUNT CONSTRAINTS
- `init`: Creates new account via System Program CPI
- `payer`: Specifies rent-paying account
- `mint::decimals`: Sets token decimal places
- `mint::authority`: Sets account authorized to mint tokens (required)
- `mint::freeze_authority`: Sets account authorized to freeze token accounts (optional)
- `seeds` + `bump`: Optional - creates mint at a Program Derived Address (PDA)

## CREATION APPROACHES

### 1. USING GENERATED KEYPAIR
- USAGE: When unique, non-deterministic mint addresses are acceptable
- IMPLEMENTATION:
  ```rust
  #[derive(Accounts)]
  pub struct CreateMint<'info> {
      #[account(mut)]
      pub signer: Signer<'info>,
      #[account(
          init,
          payer = signer,
          mint::decimals = 6,
          mint::authority = signer.key(),
          mint::freeze_authority = signer.key(),
      )]
      pub mint: InterfaceAccount<'info, Mint>,
      pub token_program: Interface<'info, TokenInterface>,
      pub system_program: Program<'info, System>,
  }
  ```
- CLIENT USAGE:
  ```typescript
  // Generate keypair for mint
  const mint = anchor.web3.Keypair.generate();
  
  // Pass keypair as signer
  await program.methods
    .createMint()
    .accounts({
      mint: mint.publicKey,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .signers([mint])
    .rpc();
  ```

### 2. USING PROGRAM DERIVED ADDRESS (PDA)
- USAGE: When deterministic, derivable mint addresses are needed
- IMPLEMENTATION:
  ```rust
  #[derive(Accounts)]
  pub struct CreateMint<'info> {
      #[account(mut)]
      pub signer: Signer<'info>,
      #[account(
          init,
          payer = signer,
          mint::decimals = 6,
          mint::authority = mint.key(),
          mint::freeze_authority = mint.key(),
          seeds = [b"mint"],
          bump
      )]
      pub mint: InterfaceAccount<'info, Mint>,
      pub token_program: Interface<'info, TokenInterface>,
      pub system_program: Program<'info, System>,
  }
  ```
- CLIENT USAGE:
  ```typescript
  // Derive PDA for mint
  const [mint, bump] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("mint")],
    program.programId,
  );
  
  // No need to pass mint as signer
  await program.methods
    .createMint()
    .accounts({
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .rpc();
  ```

## ADVANCED PATTERNS
- SELF-AUTHORITY: Set mint/freeze authority to the mint account's own address
- PDA AUTHORITY: Use a PDA as the mint authority to enable program-controlled minting
- COMBINED APPROACH: Same PDA can be both mint account address and mint authority
</PART2>
<PART 3 - RAW CODE EXAMPLE>
>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/errors.rs
use anchor_lang::prelude::*;

#[error_code]
pub enum TutorialError {
    #[msg("Invalid fee value")]
    InvalidFee,

    #[msg("Invalid mint for the pool")]
    InvalidMint,

    #[msg("Depositing too little liquidity")]
    DepositTooSmall,

    #[msg("Output is below the minimum expected")]
    OutputTooSmall,

    #[msg("Invariant does not hold")]
    InvariantViolated,
}

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/state.rs
use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Amm {
    /// The primary key of the AMM
    pub id: Pubkey,

    /// Account that has admin authority over the AMM
    pub admin: Pubkey,

    /// The LP fee taken on each trade, in basis points
    pub fee: u16,
}

impl Amm {
    pub const LEN: usize = 8 + 32 + 32 + 2;
}

#[account]
#[derive(Default)]
pub struct Pool {
    /// Primary key of the AMM
    pub amm: Pubkey,

    /// Mint of token A
    pub mint_a: Pubkey,

    /// Mint of token B
    pub mint_b: Pubkey,
}

impl Pool {
    pub const LEN: usize = 8 + 32 + 32 + 32;
}

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

mod constants;
mod errors;
mod instructions;
mod state;

// Set the correct key here
declare_id!("AsGVFxWqEn8icRBFQApxJe68x3r9zvfSbmiEzYFATGYn");

#[program]
pub mod swap_example {
    pub use super::instructions::*;
    use super::*;

    pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
        instructions::create_amm(ctx, id, fee)
    }

    pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
        instructions::create_pool(ctx)
    }

    pub fn deposit_liquidity(
        ctx: Context<DepositLiquidity>,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
        instructions::deposit_liquidity(ctx, amount_a, amount_b)
    }

    pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
        instructions::withdraw_liquidity(ctx, amount)
    }

    pub fn swap_exact_tokens_for_tokens(
        ctx: Context<SwapExactTokensForTokens>,
        swap_a: bool,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<()> {
        instructions::swap_exact_tokens_for_tokens(ctx, swap_a, input_amount, min_output_amount)
    }
}

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/constants.rs
use anchor_lang::prelude::*;

#[constant]
pub const MINIMUM_LIQUIDITY: u64 = 100;

#[constant]
pub const AUTHORITY_SEED: &[u8] = b"authority";

#[constant]
pub const LIQUIDITY_SEED: &[u8] = b"liquidity";

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/instructions/withdraw_liquidity.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, Mint, Token, TokenAccount, Transfer},
};
use fixed::types::I64F64;

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED, MINIMUM_LIQUIDITY},
    state::{Amm, Pool},
};

pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds = &[
        &ctx.accounts.pool.amm.to_bytes(),
        &ctx.accounts.mint_a.key().to_bytes(),
        &ctx.accounts.mint_b.key().to_bytes(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];
    let signer_seeds = &[&authority_seeds[..]];

    // Transfer tokens from the pool
    let amount_a = I64F64::from_num(amount)
        .checked_mul(I64F64::from_num(ctx.accounts.pool_account_a.amount))
        .unwrap()
        .checked_div(I64F64::from_num(
            ctx.accounts.mint_liquidity.supply + MINIMUM_LIQUIDITY,
        ))
        .unwrap()
        .floor()
        .to_num::<u64>();
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_account_a.to_account_info(),
                to: ctx.accounts.depositor_account_a.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount_a,
    )?;

    let amount_b = I64F64::from_num(amount)
        .checked_mul(I64F64::from_num(ctx.accounts.pool_account_b.amount))
        .unwrap()
        .checked_div(I64F64::from_num(
            ctx.accounts.mint_liquidity.supply + MINIMUM_LIQUIDITY,
        ))
        .unwrap()
        .floor()
        .to_num::<u64>();
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_account_b.to_account_info(),
                to: ctx.accounts.depositor_account_b.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        amount_b,
    )?;

    // Burn the liquidity tokens
    // It will fail if the amount is invalid
    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint_liquidity.to_account_info(),
                from: ctx.accounts.depositor_account_liquidity.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawLiquidity<'info> {
    #[account(
        seeds = [
            amm.id.as_ref()
        ],
        bump,
    )]
    pub amm: Account<'info, Amm>,

    #[account(
        seeds = [
            pool.amm.as_ref(),
            pool.mint_a.key().as_ref(),
            pool.mint_b.key().as_ref(),
        ],
        bump,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: Read only authority
    #[account(
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    /// The account paying for all rents
    pub depositor: Signer<'info>,

    #[account(
        mut,
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED,
        ],
        bump,
    )]
    pub mint_liquidity: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub mint_a: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub mint_b: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_liquidity,
        associated_token::authority = depositor,
    )]
    pub depositor_account_liquidity: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = depositor,
    )]
    pub depositor_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = depositor,
    )]
    pub depositor_account_b: Box<Account<'info, TokenAccount>>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/instructions/swap_exact_tokens_for_tokens.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};
use fixed::types::I64F64;

use crate::{
    constants::AUTHORITY_SEED,
    errors::*,
    state::{Amm, Pool},
};

pub fn swap_exact_tokens_for_tokens(
    ctx: Context<SwapExactTokensForTokens>,
    swap_a: bool,
    input_amount: u64,
    min_output_amount: u64,
) -> Result<()> {
    // Prevent depositing assets the depositor does not own
    let input = if swap_a && input_amount > ctx.accounts.trader_account_a.amount {
        ctx.accounts.trader_account_a.amount
    } else if !swap_a && input_amount > ctx.accounts.trader_account_b.amount {
        ctx.accounts.trader_account_b.amount
    } else {
        input_amount
    };

    // Apply trading fee, used to compute the output
    let amm = &ctx.accounts.amm;
    let taxed_input = input - input * amm.fee as u64 / 10000;

    let pool_a = &ctx.accounts.pool_account_a;
    let pool_b = &ctx.accounts.pool_account_b;
    let output = if swap_a {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_b.amount))
            .unwrap()
            .checked_div(
                I64F64::from_num(pool_a.amount)
                    .checked_add(I64F64::from_num(taxed_input))
                    .unwrap(),
            )
            .unwrap()
    } else {
        I64F64::from_num(taxed_input)
            .checked_mul(I64F64::from_num(pool_a.amount))
            .unwrap()
            .checked_div(
                I64F64::from_num(pool_b.amount)
                    .checked_add(I64F64::from_num(taxed_input))
                    .unwrap(),
            )
            .unwrap()
    }
    .to_num::<u64>();

    if output < min_output_amount {
        return err!(TutorialError::OutputTooSmall);
    }

    // Compute the invariant before the trade
    let invariant = pool_a.amount * pool_b.amount;

    // Transfer tokens to the pool
    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds = &[
        &ctx.accounts.pool.amm.to_bytes(),
        &ctx.accounts.mint_a.key().to_bytes(),
        &ctx.accounts.mint_b.key().to_bytes(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];
    let signer_seeds = &[&authority_seeds[..]];
    if swap_a {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.trader_account_a.to_account_info(),
                    to: ctx.accounts.pool_account_a.to_account_info(),
                    authority: ctx.accounts.trader.to_account_info(),
                },
            ),
            input,
        )?;
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_account_b.to_account_info(),
                    to: ctx.accounts.trader_account_b.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            output,
        )?;
    } else {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_account_a.to_account_info(),
                    to: ctx.accounts.trader_account_a.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer_seeds,
            ),
            input,
        )?;
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.trader_account_b.to_account_info(),
                    to: ctx.accounts.pool_account_b.to_account_info(),
                    authority: ctx.accounts.trader.to_account_info(),
                },
            ),
            output,
        )?;
    }

    msg!(
        "Traded {} tokens ({} after fees) for {}",
        input,
        taxed_input,
        output
    );

    // Verify the invariant still holds
    // Reload accounts because of the CPIs
    // We tolerate if the new invariant is higher because it means a rounding error for LPs
    ctx.accounts.pool_account_a.reload()?;
    ctx.accounts.pool_account_b.reload()?;
    if invariant > ctx.accounts.pool_account_a.amount * ctx.accounts.pool_account_a.amount {
        return err!(TutorialError::InvariantViolated);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct SwapExactTokensForTokens<'info> {
    #[account(
        seeds = [
            amm.id.as_ref()
        ],
        bump,
    )]
    pub amm: Account<'info, Amm>,

    #[account(
        seeds = [
            pool.amm.as_ref(),
            pool.mint_a.key().as_ref(),
            pool.mint_b.key().as_ref(),
        ],
        bump,
        has_one = amm,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: Read only authority
    #[account(
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    /// The account doing the swap
    pub trader: Signer<'info>,

    pub mint_a: Box<Account<'info, Mint>>,

    pub mint_b: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_b: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = trader,
    )]
    pub trader_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = trader,
    )]
    pub trader_account_b: Box<Account<'info, TokenAccount>>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/instructions/create_pool.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED},
    state::{Amm, Pool},
};

pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    pool.amm = ctx.accounts.amm.key();
    pool.mint_a = ctx.accounts.mint_a.key();
    pool.mint_b = ctx.accounts.mint_b.key();

    Ok(())
}

#[derive(Accounts)]
pub struct CreatePool<'info> {
    #[account(
        seeds = [
            amm.id.as_ref()
        ],
        bump,
    )]
    pub amm: Box<Account<'info, Amm>>,

    #[account(
        init,
        payer = payer,
        space = Pool::LEN,
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
        ],
        bump,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: Read only authority
    #[account(
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [
            amm.key().as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED,
        ],
        bump,
        mint::decimals = 6,
        mint::authority = pool_authority,
    )]
    pub mint_liquidity: Box<Account<'info, Mint>>,

    pub mint_a: Box<Account<'info, Mint>>,

    pub mint_b: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_b: Box<Account<'info, TokenAccount>>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/instructions/mod.rs
mod create_amm;
mod create_pool;
mod deposit_liquidity;
mod swap_exact_tokens_for_tokens;
mod withdraw_liquidity;

pub use create_amm::*;
pub use create_pool::*;
pub use deposit_liquidity::*;
pub use swap_exact_tokens_for_tokens::*;
pub use withdraw_liquidity::*;

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/instructions/create_amm.rs
use anchor_lang::prelude::*;

use crate::{errors::*, state::Amm};

pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
    let amm = &mut ctx.accounts.amm;
    amm.id = id;
    amm.admin = ctx.accounts.admin.key();
    amm.fee = fee;

    Ok(())
}

#[derive(Accounts)]
#[instruction(id: Pubkey, fee: u16)]
pub struct CreateAmm<'info> {
    #[account(
        init,
        payer = payer,
        space = Amm::LEN,
        seeds = [
            id.as_ref()
        ],
        bump,
        constraint = fee < 10000 @ TutorialError::InvalidFee,
    )]
    pub amm: Account<'info, Amm>,

    /// The admin of the AMM
    /// CHECK: Read only, delegatable creation
    pub admin: AccountInfo<'info>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-swap/anchor/programs/token-swap/src/instructions/deposit_liquidity.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount, Transfer},
};
use fixed::types::I64F64;

use crate::{
    constants::{AUTHORITY_SEED, LIQUIDITY_SEED, MINIMUM_LIQUIDITY},
    errors::TutorialError,
    state::Pool,
};

pub fn deposit_liquidity(
    ctx: Context<DepositLiquidity>,
    amount_a: u64,
    amount_b: u64,
) -> Result<()> {
    // Prevent depositing assets the depositor does not own
    let mut amount_a = if amount_a > ctx.accounts.depositor_account_a.amount {
        ctx.accounts.depositor_account_a.amount
    } else {
        amount_a
    };
    let mut amount_b = if amount_b > ctx.accounts.depositor_account_b.amount {
        ctx.accounts.depositor_account_b.amount
    } else {
        amount_b
    };

    // Making sure they are provided in the same proportion as existing liquidity
    let pool_a = &ctx.accounts.pool_account_a;
    let pool_b = &ctx.accounts.pool_account_b;
    // Defining pool creation like this allows attackers to frontrun pool creation with bad ratios
    let pool_creation = pool_a.amount == 0 && pool_b.amount == 0;
    (amount_a, amount_b) = if pool_creation {
        // Add as is if there is no liquidity
        (amount_a, amount_b)
    } else {
        let ratio = I64F64::from_num(pool_a.amount)
            .checked_mul(I64F64::from_num(pool_b.amount))
            .unwrap();
        if pool_a.amount > pool_b.amount {
            (
                I64F64::from_num(amount_b)
                    .checked_mul(ratio)
                    .unwrap()
                    .to_num::<u64>(),
                amount_b,
            )
        } else {
            (
                amount_a,
                I64F64::from_num(amount_a)
                    .checked_div(ratio)
                    .unwrap()
                    .to_num::<u64>(),
            )
        }
    };

    // Computing the amount of liquidity about to be deposited
    let mut liquidity = I64F64::from_num(amount_a)
        .checked_mul(I64F64::from_num(amount_b))
        .unwrap()
        .sqrt()
        .to_num::<u64>();

    // Lock some minimum liquidity on the first deposit
    if pool_creation {
        if liquidity < MINIMUM_LIQUIDITY {
            return err!(TutorialError::DepositTooSmall);
        }

        liquidity -= MINIMUM_LIQUIDITY;
    }

    // Transfer tokens to the pool
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.depositor_account_a.to_account_info(),
                to: ctx.accounts.pool_account_a.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount_a,
    )?;
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.depositor_account_b.to_account_info(),
                to: ctx.accounts.pool_account_b.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        amount_b,
    )?;

    // Mint the liquidity to user
    let authority_bump = ctx.bumps.pool_authority;
    let authority_seeds = &[
        &ctx.accounts.pool.amm.to_bytes(),
        &ctx.accounts.mint_a.key().to_bytes(),
        &ctx.accounts.mint_b.key().to_bytes(),
        AUTHORITY_SEED,
        &[authority_bump],
    ];
    let signer_seeds = &[&authority_seeds[..]];
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint_liquidity.to_account_info(),
                to: ctx.accounts.depositor_account_liquidity.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
            signer_seeds,
        ),
        liquidity,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct DepositLiquidity<'info> {
    #[account(
        seeds = [
            pool.amm.as_ref(),
            pool.mint_a.key().as_ref(),
            pool.mint_b.key().as_ref(),
        ],
        bump,
        has_one = mint_a,
        has_one = mint_b,
    )]
    pub pool: Box<Account<'info, Pool>>,

    /// CHECK: Read only authority
    #[account(
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            AUTHORITY_SEED,
        ],
        bump,
    )]
    pub pool_authority: AccountInfo<'info>,

    /// The account paying for all rents
    pub depositor: Signer<'info>,

    #[account(
        mut,
        seeds = [
            pool.amm.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            LIQUIDITY_SEED,
        ],
        bump,
    )]
    pub mint_liquidity: Box<Account<'info, Mint>>,

    pub mint_a: Box<Account<'info, Mint>>,

    pub mint_b: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = pool_authority,
    )]
    pub pool_account_b: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_liquidity,
        associated_token::authority = depositor,
    )]
    pub depositor_account_liquidity: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = depositor,
    )]
    pub depositor_account_a: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = depositor,
    )]
    pub depositor_account_b: Box<Account<'info, TokenAccount>>,

    /// The account paying for all rents
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Solana ecosystem accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/nft-operations/anchor/programs/mint-nft/src/lib.rs
use anchor_lang::prelude::*;

declare_id!("3EMcczaGi9ivdLxvvFwRbGYeEUEHpGwabXegARw4jLxa");

pub mod contexts;

pub use contexts::*;

#[program]
pub mod mint_nft {

    use super::*;
    pub fn create_collection(ctx: Context<CreateCollection>) -> Result<()> {
        ctx.accounts.create_collection(&ctx.bumps)
    }
    
    pub fn mint_nft(ctx: Context<MintNFT>) -> Result<()> {
        ctx.accounts.mint_nft(&ctx.bumps)
    }

    pub fn verify_collection(ctx: Context<VerifyCollectionMint>) -> Result<()> {
        ctx.accounts.verify_collection(&ctx.bumps)
    }
}

>>> program-examples/tokens/transfer-tokens/anchor/programs/transfer-tokens/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

pub mod instructions;

use instructions::*;

declare_id!("nHi9DdNjuupjQ3c8AJU9sChB5gLbZvTLsJQouY4hU67");

#[program]
pub mod transfer_tokens {
    use super::*;

    pub fn create_token(
        ctx: Context<CreateToken>,
        token_title: String,
        token_symbol: String,
        token_uri: String,
    ) -> Result<()> {
        create::create_token(ctx, token_title, token_symbol, token_uri)
    }

    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        mint::mint_token(ctx, amount)
    }

    pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
        transfer::transfer_tokens(ctx, amount)
    }
}

>>> program-examples/tokens/pda-mint-authority/anchor/programs/token-minter/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use instructions::*;
pub mod instructions;

declare_id!("3LFrPHqwk5jMrmiz48BFj6NV2k4NjobgTe1jChzx3JGD");

#[program]
pub mod token_minter {
    use super::*;

    pub fn create_token(
        ctx: Context<CreateToken>,
        token_name: String,
        token_symbol: String,
        token_uri: String,
    ) -> Result<()> {
        create::create_token(ctx, token_name, token_symbol, token_uri)
    }

    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        mint::mint_token(ctx, amount)
    }
}

>>> program-examples/tokens/create-token/anchor/programs/create-token/src/lib.rs
#![allow(clippy::result_large_err)]

use {
    anchor_lang::prelude::*,
    anchor_spl::{
        metadata::{
            create_metadata_accounts_v3, mpl_token_metadata::types::DataV2,
            CreateMetadataAccountsV3, Metadata,
        },
        token::{Mint, Token},
    },
};

declare_id!("GwvQ53QTu1xz3XXYfG5m5jEqwhMBvVBudPS8TUuFYnhT");

#[program]
pub mod create_token {
    use super::*;

    pub fn create_token_mint(
        ctx: Context<CreateTokenMint>,
        _token_decimals: u8,
        token_name: String,
        token_symbol: String,
        token_uri: String,
    ) -> Result<()> {
        msg!("Creating metadata account...");
        msg!(
            "Metadata account address: {}",
            &ctx.accounts.metadata_account.key()
        );

        // Cross Program Invocation (CPI)
        // Invoking the create_metadata_account_v3 instruction on the token metadata program
        create_metadata_accounts_v3(
            CpiContext::new(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: ctx.accounts.metadata_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    mint_authority: ctx.accounts.payer.to_account_info(),
                    update_authority: ctx.accounts.payer.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            ),
            DataV2 {
                name: token_name,
                symbol: token_symbol,
                uri: token_uri,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            false, // Is mutable
            true,  // Update authority is signer
            None,  // Collection details
        )?;

        msg!("Token mint created successfully.");

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(_token_decimals: u8)]
pub struct CreateTokenMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,
    // Create new mint account
    #[account(
        init,
        payer = payer,
        mint::decimals = _token_decimals,
        mint::authority = payer.key(),
    )]
    pub mint_account: Account<'info, Mint>,

    pub token_metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

>>> program-examples/tokens/nft-operations/anchor/programs/mint-nft/src/contexts/mod.rs
pub mod mint_nft;
pub mod create_collection;
pub mod verify_collection;

pub use mint_nft::*;
pub use create_collection::*;
pub use verify_collection::*;

>>> program-examples/tokens/nft-operations/anchor/programs/mint-nft/src/contexts/create_collection.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    metadata::Metadata, 
    token::{
        mint_to, 
        Mint, 
        MintTo, 
        Token, 
        TokenAccount,
    }
};
use anchor_spl::metadata::mpl_token_metadata::{
    instructions::{
        CreateMasterEditionV3Cpi, 
        CreateMasterEditionV3CpiAccounts, 
        CreateMasterEditionV3InstructionArgs, 
        CreateMetadataAccountV3Cpi, 
        CreateMetadataAccountV3CpiAccounts, 
        CreateMetadataAccountV3InstructionArgs
    }, 
    types::{
        CollectionDetails, 
        Creator, 
        DataV2
    }
};

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    user: Signer<'info>,
    #[account(
        init,
        payer = user,
        mint::decimals = 0,
        mint::authority = mint_authority,
        mint::freeze_authority = mint_authority,
    )]
    mint: Account<'info, Mint>,
    #[account(
        seeds = [b"authority"],
        bump,
    )]
    /// CHECK: This account is not initialized and is being used for signing purposes only
    pub mint_authority: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    metadata: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    master_edition: UncheckedAccount<'info>,
    #[account(
        init,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user
    )]
    destination: Account<'info, TokenAccount>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    token_metadata_program: Program<'info, Metadata>,
}

impl<'info> CreateCollection<'info> {
    pub fn create_collection(&mut self, bumps: &CreateCollectionBumps) -> Result<()> {

        let metadata = &self.metadata.to_account_info();
        let master_edition = &self.master_edition.to_account_info();
        let mint = &self.mint.to_account_info();
        let authority = &self.mint_authority.to_account_info();
        let payer = &self.user.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let spl_token_program = &self.token_program.to_account_info();
        let spl_metadata_program = &self.token_metadata_program.to_account_info();

        let seeds = &[
            &b"authority"[..], 
            &[bumps.mint_authority]
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = MintTo {
            mint: self.mint.to_account_info(),
            to: self.destination.to_account_info(),
            authority: self.mint_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        mint_to(cpi_ctx, 1)?;
        msg!("Collection NFT minted!");

        let creator = vec![
            Creator {
                address: self.mint_authority.key().clone(),
                verified: true,
                share: 100,
            },
        ];
        
        let metadata_account = CreateMetadataAccountV3Cpi::new(
            spl_metadata_program, 
            CreateMetadataAccountV3CpiAccounts {
                metadata,
                mint,
                mint_authority: authority,
                payer,
                update_authority: (authority, true),
                system_program,
                rent: None,
            },
            CreateMetadataAccountV3InstructionArgs {
                data: DataV2 {
                    name: "DummyCollection".to_owned(),
                    symbol: "DC".to_owned(),
                    uri: "".to_owned(),
                    seller_fee_basis_points: 0,
                    creators: Some(creator),
                    collection: None,
                    uses: None,
                },
                is_mutable: true,
                collection_details: Some(
                    CollectionDetails::V1 { 
                        size: 0 
                    }
                )
            }
        );
        metadata_account.invoke_signed(signer_seeds)?;
        msg!("Metadata Account created!");

        let master_edition_account = CreateMasterEditionV3Cpi::new(
            spl_metadata_program,
            CreateMasterEditionV3CpiAccounts {
                edition: master_edition,
                update_authority: authority,
                mint_authority: authority,
                mint,
                payer,
                metadata,
                token_program: spl_token_program,
                system_program,
                rent: None,
            },
            CreateMasterEditionV3InstructionArgs {
                max_supply: Some(0),
            }
        );
        master_edition_account.invoke_signed(signer_seeds)?;
        msg!("Master Edition Account created");
        
        Ok(())
    }
}
>>> program-examples/tokens/nft-operations/anchor/programs/mint-nft/src/contexts/mint_nft.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    metadata::Metadata, 
    token::{
        mint_to,
        Mint, 
        MintTo, 
        Token, 
        TokenAccount
    }
};
use anchor_spl::metadata::mpl_token_metadata::{
    instructions::{
        CreateMasterEditionV3Cpi, 
        CreateMasterEditionV3CpiAccounts, 
        CreateMasterEditionV3InstructionArgs, 
        CreateMetadataAccountV3Cpi, 
        CreateMetadataAccountV3CpiAccounts, 
        CreateMetadataAccountV3InstructionArgs,
    }, 
    types::{
        Collection, 
        Creator, 
        DataV2,
    }
};

#[derive(Accounts)]
pub struct MintNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        mint::decimals = 0,
        mint::authority = mint_authority,
        mint::freeze_authority = mint_authority,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = owner
    )]
    pub destination: Account<'info, TokenAccount>,
    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    pub metadata: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    pub master_edition: UncheckedAccount<'info>,
    #[account(
        seeds = [b"authority"],
        bump,
    )]
    /// CHECK: This is account is not initialized and is being used for signing purposes only
    pub mint_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub collection_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metadata>,
}

impl<'info> MintNFT<'info> {
    pub fn mint_nft(&mut self, bumps: &MintNFTBumps) -> Result<()> {

        let metadata = &self.metadata.to_account_info();
        let master_edition = &self.master_edition.to_account_info();
        let mint = &self.mint.to_account_info();
        let authority = &self.mint_authority.to_account_info();
        let payer = &self.owner.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let spl_token_program = &self.token_program.to_account_info();
        let spl_metadata_program = &self.token_metadata_program.to_account_info();

        let seeds = &[
            &b"authority"[..], 
            &[bumps.mint_authority]
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = MintTo {
            mint: self.mint.to_account_info(),
            to: self.destination.to_account_info(),
            authority: self.mint_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        mint_to(cpi_ctx, 1)?;
        msg!("Collection NFT minted!");

        let creator = vec![
            Creator {
                address: self.mint_authority.key(),
                verified: true,
                share: 100,
            },
        ];

        let metadata_account = CreateMetadataAccountV3Cpi::new(
            spl_metadata_program,
            CreateMetadataAccountV3CpiAccounts {
                metadata,
                mint,
                mint_authority: authority,
                payer,
                update_authority: (authority, true),
                system_program,
                rent: None,
            }, 
            CreateMetadataAccountV3InstructionArgs {
                data: DataV2 {
                    name: "Mint Test".to_string(),
                    symbol: "YAY".to_string(),
                    uri: "".to_string(),
                    seller_fee_basis_points: 0,
                    creators: Some(creator),
                    collection: Some(Collection {
                        verified: false,
                        key: self.collection_mint.key(),
                    }),
                    uses: None
                },
                is_mutable: true,
                collection_details: None,
            }
        );
        metadata_account.invoke_signed(signer_seeds)?;

        let master_edition_account = CreateMasterEditionV3Cpi::new(
            spl_metadata_program,
            CreateMasterEditionV3CpiAccounts {
                edition: master_edition,
                update_authority: authority,
                mint_authority: authority,
                mint,
                payer,
                metadata,
                token_program: spl_token_program,
                system_program,
                rent: None,
            },
            CreateMasterEditionV3InstructionArgs {
                max_supply: Some(0),
            }
        );
        master_edition_account.invoke_signed(signer_seeds)?;

        Ok(())
        
    }
}
>>> program-examples/tokens/nft-operations/anchor/programs/mint-nft/src/contexts/verify_collection.rs
use anchor_lang::prelude::*;

use anchor_spl::metadata::mpl_token_metadata::instructions::{
    VerifyCollectionV1Cpi,
    VerifyCollectionV1CpiAccounts,
};
use anchor_spl::metadata::{
    MasterEditionAccount, 
    MetadataAccount,
};
use anchor_spl::{
    token::Mint, 
    metadata::Metadata, 
};
pub use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;

#[derive(Accounts)]
pub struct VerifyCollectionMint<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub metadata: Account<'info, MetadataAccount>,
    pub mint: Account<'info, Mint>,
    #[account(
        seeds = [b"authority"],
        bump,
    )]
    /// CHECK: This account is not initialized and is being used for signing purposes only
    pub mint_authority: UncheckedAccount<'info>,
    pub collection_mint: Account<'info, Mint>,
    #[account(mut)]
    pub collection_metadata: Account<'info, MetadataAccount>,
    pub collection_master_edition: Account<'info, MasterEditionAccount>,
    pub system_program: Program<'info, System>,
    #[account(address = INSTRUCTIONS_ID)]
    /// CHECK: Sysvar instruction account that is being checked with an address constraint
    pub sysvar_instruction: UncheckedAccount<'info>,
    pub token_metadata_program: Program<'info, Metadata>,
}

impl<'info> VerifyCollectionMint<'info> {
    pub fn verify_collection(&mut self, bumps: &VerifyCollectionMintBumps) -> Result<()> {
        let metadata = &self.metadata.to_account_info();
        let authority = &self.mint_authority.to_account_info();
        let collection_mint = &self.collection_mint.to_account_info();
        let collection_metadata = &self.collection_metadata.to_account_info();
        let collection_master_edition = &self.collection_master_edition.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let sysvar_instructions = &self.sysvar_instruction.to_account_info();
        let spl_metadata_program = &self.token_metadata_program.to_account_info();

        let seeds = &[
            &b"authority"[..], 
            &[bumps.mint_authority]
        ];
        let signer_seeds = &[&seeds[..]];

        let verify_collection = VerifyCollectionV1Cpi::new(
            spl_metadata_program,
        VerifyCollectionV1CpiAccounts {
            authority,
            delegate_record: None,
            metadata,
            collection_mint,
            collection_metadata: Some(collection_metadata),
            collection_master_edition: Some(collection_master_edition),
            system_program,
            sysvar_instructions,
        });
        verify_collection.invoke_signed(signer_seeds)?;

        msg!("Collection Verified!");
        
        Ok(())
    }
}
>>> program-examples/tokens/nft-minter/anchor/programs/nft-minter/src/lib.rs
#![allow(clippy::result_large_err)]

use {
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        metadata::{
            create_master_edition_v3, create_metadata_accounts_v3,
            mpl_token_metadata::types::DataV2, CreateMasterEditionV3, CreateMetadataAccountsV3,
            Metadata,
        },
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

declare_id!("52quezNUzc1Ej6Jh6L4bvtxPW8j6TEFHuLVAWiFvdnsc");

#[program]
pub mod nft_minter {
    use super::*;

    pub fn mint_nft(
        ctx: Context<CreateToken>,
        nft_name: String,
        nft_symbol: String,
        nft_uri: String,
    ) -> Result<()> {
        msg!("Minting Token");
        // Cross Program Invocation (CPI)
        // Invoking the mint_to instruction on the token program
        mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint_account.to_account_info(),
                    to: ctx.accounts.associated_token_account.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            1,
        )?;

        msg!("Creating metadata account");
        // Cross Program Invocation (CPI)
        // Invoking the create_metadata_account_v3 instruction on the token metadata program
        create_metadata_accounts_v3(
            CpiContext::new(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: ctx.accounts.metadata_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    mint_authority: ctx.accounts.payer.to_account_info(),
                    update_authority: ctx.accounts.payer.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            ),
            DataV2 {
                name: nft_name,
                symbol: nft_symbol,
                uri: nft_uri,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            false, // Is mutable
            true,  // Update authority is signer
            None,  // Collection details
        )?;

        msg!("Creating master edition account");
        // Cross Program Invocation (CPI)
        // Invoking the create_master_edition_v3 instruction on the token metadata program
        create_master_edition_v3(
            CpiContext::new(
                ctx.accounts.token_metadata_program.to_account_info(),
                CreateMasterEditionV3 {
                    edition: ctx.accounts.edition_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    update_authority: ctx.accounts.payer.to_account_info(),
                    mint_authority: ctx.accounts.payer.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    metadata: ctx.accounts.metadata_account.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            ),
            None, // Max Supply
        )?;

        msg!("NFT minted successfully.");

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref(), b"edition"],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub edition_account: UncheckedAccount<'info>,

    // Create new mint account, NFTs have 0 decimals
    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = payer.key(),
        mint::freeze_authority = payer.key(),
    )]
    pub mint_account: Account<'info, Mint>,

    // Create associated token account, if needed
    // This is the account that will hold the NFT
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_account,
        associated_token::authority = payer,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

>>> program-examples/tokens/spl-token-minter/anchor/programs/spl-token-minter/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

pub mod instructions;
use instructions::*;

declare_id!("3of89Z9jwek9zrFgpCWc9jZvQvitpVMxpZNsrAD2vQUD");

#[program]
pub mod spl_token_minter {
    use super::*;

    pub fn create_token(
        ctx: Context<CreateToken>,
        token_name: String,
        token_symbol: String,
        token_uri: String,
    ) -> Result<()> {
        create::create_token(ctx, token_name, token_symbol, token_uri)
    }

    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        mint::mint_token(ctx, amount)
    }
}

>>> program-examples/tokens/pda-mint-authority/anchor/programs/token-minter/src/instructions/mint.rs
use {
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    // Mint account address is a PDA
    #[account(
        mut,
        seeds = [b"mint"],
        bump
    )]
    pub mint_account: Account<'info, Mint>,

    // Create Associated Token Account, if needed
    // This is the account that will hold the minted tokens
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint_account,
        associated_token::authority = payer,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
    msg!("Minting token to associated token account...");
    msg!("Mint: {}", &ctx.accounts.mint_account.key());
    msg!(
        "Token Address: {}",
        &ctx.accounts.associated_token_account.key()
    );

    // PDA signer seeds
    let signer_seeds: &[&[&[u8]]] = &[&[b"mint", &[ctx.bumps.mint_account]]];

    // Invoke the mint_to instruction on the token program
    mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint_account.to_account_info(),
                to: ctx.accounts.associated_token_account.to_account_info(),
                authority: ctx.accounts.mint_account.to_account_info(), // PDA mint authority, required as signer
            },
        )
        .with_signer(signer_seeds), // using PDA to sign
        amount * 10u64.pow(ctx.accounts.mint_account.decimals as u32), // Mint tokens, adjust for decimals
    )?;

    msg!("Token minted successfully.");

    Ok(())
}

>>> program-examples/tokens/pda-mint-authority/anchor/programs/token-minter/src/instructions/mod.rs
pub mod create;
pub mod mint;

pub use create::*;
pub use mint::*;

>>> program-examples/tokens/pda-mint-authority/anchor/programs/token-minter/src/instructions/create.rs
// In this example the same PDA is used as both the address of the mint account and the mint authority
// This is to demonstrate that the same PDA can be used for both the address of an account and CPI signing
use {
    anchor_lang::prelude::*,
    anchor_spl::{
        metadata::{
            create_metadata_accounts_v3, mpl_token_metadata::types::DataV2,
            CreateMetadataAccountsV3, Metadata,
        },
        token::{Mint, Token},
    },
};

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    // Create mint account
    // Same PDA as address of the account and mint/freeze authority
    #[account(
        init,
        seeds = [b"mint"],
        bump,
        payer = payer,
        mint::decimals = 9,
        mint::authority = mint_account.key(),
        mint::freeze_authority = mint_account.key(),

    )]
    pub mint_account: Account<'info, Mint>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_token(
    ctx: Context<CreateToken>,
    token_name: String,
    token_symbol: String,
    token_uri: String,
) -> Result<()> {
    msg!("Creating metadata account");

    // PDA signer seeds
    let signer_seeds: &[&[&[u8]]] = &[&[b"mint", &[ctx.bumps.mint_account]]];

    // Cross Program Invocation (CPI) signed by PDA
    // Invoking the create_metadata_account_v3 instruction on the token metadata program
    create_metadata_accounts_v3(
        CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                mint_authority: ctx.accounts.mint_account.to_account_info(), // PDA is mint authority
                update_authority: ctx.accounts.mint_account.to_account_info(), // PDA is update authority
                payer: ctx.accounts.payer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        )
        .with_signer(signer_seeds),
        DataV2 {
            name: token_name,
            symbol: token_symbol,
            uri: token_uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        false, // Is mutable
        true,  // Update authority is signer
        None,  // Collection details
    )?;

    msg!("Token created successfully.");

    Ok(())
}

>>> program-examples/tokens/transfer-tokens/anchor/programs/transfer-tokens/src/instructions/transfer.rs
use {
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{transfer, Mint, Token, TokenAccount, Transfer},
    },
};

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    pub recipient: SystemAccount<'info>,

    #[account(mut)]
    pub mint_account: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = sender,
    )]
    pub sender_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = sender,
        associated_token::mint = mint_account,
        associated_token::authority = recipient,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
    msg!("Transferring tokens...");
    msg!(
        "Mint: {}",
        &ctx.accounts.mint_account.to_account_info().key()
    );
    msg!(
        "From Token Address: {}",
        &ctx.accounts.sender_token_account.key()
    );
    msg!(
        "To Token Address: {}",
        &ctx.accounts.recipient_token_account.key()
    );

    // Invoke the transfer instruction on the token program
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.sender_token_account.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.sender.to_account_info(),
            },
        ),
        amount * 10u64.pow(ctx.accounts.mint_account.decimals as u32), // Transfer amount, adjust for decimals
    )?;

    msg!("Tokens transferred successfully.");

    Ok(())
}

>>> program-examples/tokens/transfer-tokens/anchor/programs/transfer-tokens/src/instructions/mint.rs
use {
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub mint_authority: Signer<'info>,

    pub recipient: SystemAccount<'info>,
    #[account(mut)]
    pub mint_account: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = mint_authority,
        associated_token::mint = mint_account,
        associated_token::authority = recipient,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
    msg!("Minting tokens to associated token account...");
    msg!("Mint: {}", &ctx.accounts.mint_account.key());
    msg!(
        "Token Address: {}",
        &ctx.accounts.associated_token_account.key()
    );

    // Invoke the mint_to instruction on the token program
    mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint_account.to_account_info(),
                to: ctx.accounts.associated_token_account.to_account_info(),
                authority: ctx.accounts.mint_authority.to_account_info(),
            },
        ),
        amount * 10u64.pow(ctx.accounts.mint_account.decimals as u32), // Mint tokens
    )?;

    msg!("Token minted successfully.");

    Ok(())
}

>>> program-examples/tokens/transfer-tokens/anchor/programs/transfer-tokens/src/instructions/mod.rs
pub mod create;
pub mod mint;
pub mod transfer;

pub use create::*;
pub use mint::*;
pub use transfer::*;

>>> program-examples/tokens/transfer-tokens/anchor/programs/transfer-tokens/src/instructions/create.rs
use {
    anchor_lang::prelude::*,
    anchor_spl::{
        metadata::{
            create_metadata_accounts_v3, mpl_token_metadata::types::DataV2,
            CreateMetadataAccountsV3, Metadata,
        },
        token::{Mint, Token},
    },
};

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = 9,
        mint::authority = payer.key(),
        mint::freeze_authority = payer.key(),

    )]
    pub mint_account: Account<'info, Mint>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_token(
    ctx: Context<CreateToken>,
    token_name: String,
    token_symbol: String,
    token_uri: String,
) -> Result<()> {
    msg!("Creating metadata account");

    // Cross Program Invocation (CPI)
    // Invoking the create_metadata_account_v3 instruction on the token metadata program
    create_metadata_accounts_v3(
        CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                mint_authority: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.payer.to_account_info(),
                payer: ctx.accounts.payer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        ),
        DataV2 {
            name: token_name,
            symbol: token_symbol,
            uri: token_uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        false, // Is mutable
        true,  // Update authority is signer
        None,  // Collection details
    )?;

    msg!("Token created successfully.");

    Ok(())
}

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/state/mod.rs
pub mod offer;

pub use offer::*;

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/state/offer.rs
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub id: u64,
    pub maker: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_b_wanted_amount: u64,
    pub bump: u8,
}

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/lib.rs
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("qbuMdeYxYJXBjU6C6qFKjZKjXmrU83eDQomHdrch826");

#[program]
pub mod escrow {
    use super::*;

    pub fn make_offer(
        context: Context<MakeOffer>,
        id: u64,
        token_a_offered_amount: u64,
        token_b_wanted_amount: u64,
    ) -> Result<()> {
        instructions::make_offer::send_offered_tokens_to_vault(&context, token_a_offered_amount)?;
        instructions::make_offer::save_offer(context, id, token_b_wanted_amount)
    }

    pub fn take_offer(context: Context<TakeOffer>) -> Result<()> {
        instructions::take_offer::send_wanted_tokens_to_maker(&context)?;
        instructions::take_offer::withdraw_and_close_vault(context)
    }
}

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/error.rs
use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Custom error message")]
    CustomError,
}

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/constants.rs
use anchor_lang::prelude::*;

#[constant]
pub const SEED: &str = "anchor";

pub const ANCHOR_DISCRIMINATOR: usize = 8;

>>> program-examples/oracles/pyth/anchor/programs/pythexample/src/lib.rs
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use anchor_lang::prelude::*;

declare_id!("GUkjQmrLPFXXNK1bFLKt8XQi6g3TjxcHVspbjDoHvMG2");

#[program]
pub mod anchor_test {
    use super::*;

    pub fn read_price(ctx: Context<ReadPrice>) -> Result<()> {
        let price_update = &ctx.accounts.price_update;
        msg!("Price feed id: {:?}", price_update.price_message.feed_id);
        msg!("Price: {:?}", price_update.price_message.price);
        msg!("Confidence: {:?}", price_update.price_message.conf);
        msg!("Exponent: {:?}", price_update.price_message.exponent);
        msg!("Publish Time: {:?}", price_update.price_message.publish_time);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ReadPrice<'info> {
    pub price_update: Account<'info, PriceUpdateV2>,
}

>>> program-examples/tokens/spl-token-minter/anchor/programs/spl-token-minter/src/instructions/mint.rs
use {
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub mint_authority: Signer<'info>,

    pub recipient: SystemAccount<'info>,
    #[account(mut)]
    pub mint_account: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = mint_authority,
        associated_token::mint = mint_account,
        associated_token::authority = recipient,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
    msg!("Minting tokens to associated token account...");
    msg!("Mint: {}", &ctx.accounts.mint_account.key());
    msg!(
        "Token Address: {}",
        &ctx.accounts.associated_token_account.key()
    );

    // Invoke the mint_to instruction on the token program
    mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint_account.to_account_info(),
                to: ctx.accounts.associated_token_account.to_account_info(),
                authority: ctx.accounts.mint_authority.to_account_info(),
            },
        ),
        amount * 10u64.pow(ctx.accounts.mint_account.decimals as u32), // Mint tokens, adjust for decimals
    )?;

    msg!("Token minted successfully.");

    Ok(())
}

>>> program-examples/tokens/spl-token-minter/anchor/programs/spl-token-minter/src/instructions/mod.rs
pub mod create;
pub mod mint;

pub use create::*;
pub use mint::*;

>>> program-examples/tokens/spl-token-minter/anchor/programs/spl-token-minter/src/instructions/create.rs
use {
    anchor_lang::prelude::*,
    anchor_spl::{
        metadata::{
            create_metadata_accounts_v3, mpl_token_metadata::types::DataV2,
            CreateMetadataAccountsV3, Metadata,
        },
        token::{Mint, Token},
    },
};

#[derive(Accounts)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = 9,
        mint::authority = payer.key(),
        mint::freeze_authority = payer.key(),

    )]
    pub mint_account: Account<'info, Mint>,
    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_token(
    ctx: Context<CreateToken>,
    token_name: String,
    token_symbol: String,
    token_uri: String,
) -> Result<()> {
    msg!("Creating metadata account");

    // Cross Program Invocation (CPI)
    // Invoking the create_metadata_account_v3 instruction on the token metadata program
    create_metadata_accounts_v3(
        CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                mint_authority: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.payer.to_account_info(),
                payer: ctx.accounts.payer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        ),
        DataV2 {
            name: token_name,
            symbol: token_symbol,
            uri: token_uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        false, // Is mutable
        true,  // Update authority is signer
        None,  // Collection details
    )?;

    msg!("Token created successfully.");

    Ok(())
}

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/instructions/make_offer.rs
use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{Offer, ANCHOR_DISCRIMINATOR};

use super::transfer_tokens;

// See https://www.anchor-lang.com/docs/account-constraints#instruction-attribute
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub token_mint_a: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub token_mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = token_mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_token_account_a: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = maker,
        space = ANCHOR_DISCRIMINATOR + Offer::INIT_SPACE,
        seeds = [b"offer", maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        init,
        payer = maker,
        associated_token::mint = token_mint_a,
        associated_token::authority = offer,
        associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

// Move the tokens from the maker's ATA to the vault
pub fn send_offered_tokens_to_vault(
    context: &Context<MakeOffer>,
    token_a_offered_amount: u64,
) -> Result<()> {
    transfer_tokens(
        &context.accounts.maker_token_account_a,
        &context.accounts.vault,
        &token_a_offered_amount,
        &context.accounts.token_mint_a,
        &context.accounts.maker,
        &context.accounts.token_program,
    )
}

// Save the details of the offer to the offer account
pub fn save_offer(context: Context<MakeOffer>, id: u64, token_b_wanted_amount: u64) -> Result<()> {
    context.accounts.offer.set_inner(Offer {
        id,
        maker: context.accounts.maker.key(),
        token_mint_a: context.accounts.token_mint_a.key(),
        token_mint_b: context.accounts.token_mint_b.key(),
        token_b_wanted_amount,
        bump: context.bumps.offer,
    });
    Ok(())
}

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/instructions/shared.rs
use anchor_lang::prelude::*;

use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

pub fn transfer_tokens<'info>(
    from: &InterfaceAccount<'info, TokenAccount>,
    to: &InterfaceAccount<'info, TokenAccount>,
    amount: &u64,
    mint: &InterfaceAccount<'info, Mint>,
    authority: &Signer<'info>,
    token_program: &Interface<'info, TokenInterface>,
) -> Result<()> {
    let transfer_accounts = TransferChecked {
        from: from.to_account_info(),
        mint: mint.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };

    let cpi_context = CpiContext::new(token_program.to_account_info(), transfer_accounts);

    transfer_checked(cpi_context, *amount, mint.decimals)
}

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/instructions/take_offer.rs
use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::Offer;

use super::transfer_tokens;

#[derive(Accounts)]
pub struct TakeOffer<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    pub token_mint_a: InterfaceAccount<'info, Mint>,

    pub token_mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = token_mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_token_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = token_mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_token_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = token_mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_token_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = token_mint_a,
        has_one = token_mint_b,
        seeds = [b"offer", maker.key().as_ref(), offer.id.to_le_bytes().as_ref()],
        bump = offer.bump
    )]
    offer: Account<'info, Offer>,

    #[account(
        mut,
        associated_token::mint = token_mint_a,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn send_wanted_tokens_to_maker(ctx: &Context<TakeOffer>) -> Result<()> {
    transfer_tokens(
        &ctx.accounts.taker_token_account_b,
        &ctx.accounts.maker_token_account_b,
        &ctx.accounts.offer.token_b_wanted_amount,
        &ctx.accounts.token_mint_b,
        &ctx.accounts.taker,
        &ctx.accounts.token_program,
    )
}

pub fn withdraw_and_close_vault(ctx: Context<TakeOffer>) -> Result<()> {
    let seeds = &[
        b"offer",
        ctx.accounts.maker.to_account_info().key.as_ref(),
        &ctx.accounts.offer.id.to_le_bytes()[..],
        &[ctx.accounts.offer.bump],
    ];
    let signer_seeds = [&seeds[..]];

    let accounts = TransferChecked {
        from: ctx.accounts.vault.to_account_info(),
        mint: ctx.accounts.token_mint_a.to_account_info(),
        to: ctx.accounts.taker_token_account_a.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
    };

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        accounts,
        &signer_seeds,
    );

    transfer_checked(
        cpi_context,
        ctx.accounts.vault.amount,
        ctx.accounts.token_mint_a.decimals,
    )?;

    let accounts = CloseAccount {
        account: ctx.accounts.vault.to_account_info(),
        destination: ctx.accounts.taker.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
    };

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        accounts,
        &signer_seeds,
    );

    close_account(cpi_context)
}

>>> program-examples/tokens/escrow/anchor/programs/escrow/src/instructions/mod.rs
pub mod make_offer;
pub use make_offer::*;

pub mod take_offer;
pub use take_offer::*;

pub mod shared;
pub use shared::*;

>>> program-examples/basics/pda-rent-payer/anchor/programs/anchor-program-example/src/lib.rs
#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use instructions::*;
pub mod instructions;

declare_id!("7Hm9nsYVuBZ9rf8z9AMUHreZRv8Q4vLhqwdVTCawRZtA");

#[program]
pub mod pda_rent_payer {
    use super::*;

    pub fn init_rent_vault(ctx: Context<InitRentVault>, fund_lamports: u64) -> Result<()> {
        init_rent_vault::init_rent_vault(ctx, fund_lamports)
    }

    pub fn create_new_account(ctx: Context<CreateNewAccount>) -> Result<()> {
        create_new_account::create_new_account(ctx)
    }
}

>>> program-examples/compression/cnft-vault/anchor/programs/cnft-vault/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use mpl_bubblegum::state::TreeConfig;
use solana_program::pubkey::Pubkey;
use spl_account_compression::{program::SplAccountCompression, Noop};

declare_id!("CNftyK7T8udPwYRzZUMWzbh79rKrz9a5GwV2wv7iEHpk");

#[derive(Clone)]
pub struct MplBubblegum;

impl anchor_lang::Id for MplBubblegum {
    fn id() -> Pubkey {
        mpl_bubblegum::id()
    }
}

// first 8 bytes of SHA256("global:transfer")
const TRANSFER_DISCRIMINATOR: &[u8; 8] = &[163, 52, 200, 231, 140, 3, 69, 186];

#[program]
pub mod cnft_vault {

    use super::*;

    pub fn withdraw_cnft<'info>(
        ctx: Context<'_, '_, '_, 'info, Withdraw<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        msg!(
            "attempting to send nft {} from tree {}",
            index,
            ctx.accounts.merkle_tree.key()
        );

        let mut accounts: Vec<solana_program::instruction::AccountMeta> = vec![
            AccountMeta::new_readonly(ctx.accounts.tree_authority.key(), false),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), true),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), false),
            AccountMeta::new_readonly(ctx.accounts.new_leaf_owner.key(), false),
            AccountMeta::new(ctx.accounts.merkle_tree.key(), false),
            AccountMeta::new_readonly(ctx.accounts.log_wrapper.key(), false),
            AccountMeta::new_readonly(ctx.accounts.compression_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        ];

        let mut data: Vec<u8> = vec![];
        data.extend(TRANSFER_DISCRIMINATOR);
        data.extend(root);
        data.extend(data_hash);
        data.extend(creator_hash);
        data.extend(nonce.to_le_bytes());
        data.extend(index.to_le_bytes());

        let mut account_infos: Vec<AccountInfo> = vec![
            ctx.accounts.tree_authority.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.new_leaf_owner.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
            ctx.accounts.log_wrapper.to_account_info(),
            ctx.accounts.compression_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ];

        // add "accounts" (hashes) that make up the merkle proof
        for acc in ctx.remaining_accounts.iter() {
            accounts.push(AccountMeta::new_readonly(acc.key(), false));
            account_infos.push(acc.to_account_info());
        }

        msg!("manual cpi call");
        solana_program::program::invoke_signed(
            &solana_program::instruction::Instruction {
                program_id: ctx.accounts.bubblegum_program.key(),
                accounts,
                data,
            },
            &account_infos[..],
            &[&[b"cNFT-vault", &[*ctx.bumps.get("leaf_owner").unwrap()]]],
        )
        .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn withdraw_two_cnfts<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawTwo<'info>>,
        root1: [u8; 32],
        data_hash1: [u8; 32],
        creator_hash1: [u8; 32],
        nonce1: u64,
        index1: u32,
        proof_1_length: u8,
        root2: [u8; 32],
        data_hash2: [u8; 32],
        creator_hash2: [u8; 32],
        nonce2: u64,
        index2: u32,
        _proof_2_length: u8, // we don't actually need this (proof_2_length = remaining_accounts_len - proof_1_length)
    ) -> Result<()> {
        let merkle_tree1 = ctx.accounts.merkle_tree1.key();
        let merkle_tree2 = ctx.accounts.merkle_tree2.key();
        msg!(
            "attempting to send nfts from trees {} and {}",
            merkle_tree1,
            merkle_tree2
        );

        // Note: in this example anyone can withdraw any NFT from the vault
        // in productions you should check if nft transfers are valid (correct NFT, correct authority)

        let mut accounts1: Vec<solana_program::instruction::AccountMeta> = vec![
            AccountMeta::new_readonly(ctx.accounts.tree_authority1.key(), false),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), true),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), false),
            AccountMeta::new_readonly(ctx.accounts.new_leaf_owner1.key(), false),
            AccountMeta::new(ctx.accounts.merkle_tree1.key(), false),
            AccountMeta::new_readonly(ctx.accounts.log_wrapper.key(), false),
            AccountMeta::new_readonly(ctx.accounts.compression_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        ];

        let mut accounts2: Vec<solana_program::instruction::AccountMeta> = vec![
            AccountMeta::new_readonly(ctx.accounts.tree_authority2.key(), false),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), true),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), false),
            AccountMeta::new_readonly(ctx.accounts.new_leaf_owner2.key(), false),
            AccountMeta::new(ctx.accounts.merkle_tree2.key(), false),
            AccountMeta::new_readonly(ctx.accounts.log_wrapper.key(), false),
            AccountMeta::new_readonly(ctx.accounts.compression_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        ];

        let mut data1: Vec<u8> = vec![];
        data1.extend(TRANSFER_DISCRIMINATOR);
        data1.extend(root1);
        data1.extend(data_hash1);
        data1.extend(creator_hash1);
        data1.extend(nonce1.to_le_bytes());
        data1.extend(index1.to_le_bytes());
        let mut data2: Vec<u8> = vec![];
        data2.extend(TRANSFER_DISCRIMINATOR);
        data2.extend(root2);
        data2.extend(data_hash2);
        data2.extend(creator_hash2);
        data2.extend(nonce2.to_le_bytes());
        data2.extend(index2.to_le_bytes());

        let mut account_infos1: Vec<AccountInfo> = vec![
            ctx.accounts.tree_authority1.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.new_leaf_owner1.to_account_info(),
            ctx.accounts.merkle_tree1.to_account_info(),
            ctx.accounts.log_wrapper.to_account_info(),
            ctx.accounts.compression_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ];
        let mut account_infos2: Vec<AccountInfo> = vec![
            ctx.accounts.tree_authority2.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.new_leaf_owner2.to_account_info(),
            ctx.accounts.merkle_tree2.to_account_info(),
            ctx.accounts.log_wrapper.to_account_info(),
            ctx.accounts.compression_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ];

        for (i, acc) in ctx.remaining_accounts.iter().enumerate() {
            if i < proof_1_length as usize {
                accounts1.push(AccountMeta::new_readonly(acc.key(), false));
                account_infos1.push(acc.to_account_info());
            } else {
                accounts2.push(AccountMeta::new_readonly(acc.key(), false));
                account_infos2.push(acc.to_account_info());
            }
        }

        msg!("withdrawing cNFT#1");
        solana_program::program::invoke_signed(
            &solana_program::instruction::Instruction {
                program_id: ctx.accounts.bubblegum_program.key(),
                accounts: accounts1,
                data: data1,
            },
            &account_infos1[..],
            &[&[b"cNFT-vault", &[*ctx.bumps.get("leaf_owner").unwrap()]]],
        )?;

        msg!("withdrawing cNFT#2");
        solana_program::program::invoke_signed(
            &solana_program::instruction::Instruction {
                program_id: ctx.accounts.bubblegum_program.key(),
                accounts: accounts2,
                data: data2,
            },
            &account_infos2[..],
            &[&[b"cNFT-vault", &[*ctx.bumps.get("leaf_owner").unwrap()]]],
        )?;

        msg!("successfully sent cNFTs");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        bump,
        seeds::program = bubblegum_program.key()
    )]
    /// CHECK: This account is neither written to nor read from.
    pub tree_authority: Account<'info, TreeConfig>,

    #[account(
        seeds = [b"cNFT-vault"],
        bump,
    )]
    /// CHECK: This account doesnt even exist (it is just the pda to sign)
    pub leaf_owner: UncheckedAccount<'info>, // sender (the vault in our case)
    /// CHECK: This account is neither written to nor read from.
    pub new_leaf_owner: UncheckedAccount<'info>, // receiver
    #[account(mut)]
    /// CHECK: This account is modified in the downstream program
    pub merkle_tree: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub bubblegum_program: Program<'info, MplBubblegum>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawTwo<'info> {
    #[account(
        seeds = [merkle_tree1.key().as_ref()],
        bump,
        seeds::program = bubblegum_program.key()
    )]
    /// CHECK: This account is neither written to nor read from.
    pub tree_authority1: Account<'info, TreeConfig>,
    #[account(
        seeds = [b"cNFT-vault"],
        bump,
    )]
    /// CHECK: This account doesnt even exist (it is just the pda to sign)
    pub leaf_owner: UncheckedAccount<'info>, // you might need two accounts if the nfts are owned by two different PDAs
    /// CHECK: This account is neither written to nor read from.
    pub new_leaf_owner1: UncheckedAccount<'info>, // receiver
    #[account(mut)]
    /// CHECK: This account is modified in the downstream program
    pub merkle_tree1: UncheckedAccount<'info>,

    #[account(
        seeds = [merkle_tree2.key().as_ref()],
        bump,
        seeds::program = bubblegum_program.key()
    )]
    /// CHECK: This account is neither written to nor read from.
    pub tree_authority2: Account<'info, TreeConfig>,
    /// CHECK: This account is neither written to nor read from.
    pub new_leaf_owner2: UncheckedAccount<'info>, // receiver
    #[account(mut)]
    /// CHECK: This account is modified in the downstream program
    pub merkle_tree2: UncheckedAccount<'info>,

    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub bubblegum_program: Program<'info, MplBubblegum>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/basics/pda-rent-payer/anchor/programs/anchor-program-example/src/instructions/init_rent_vault.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct InitRentVault<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"rent_vault",
        ],
        bump,
    )]
    rent_vault: SystemAccount<'info>,
    system_program: Program<'info, System>,
}

// When lamports are transferred to a new address (without and existing account),
// An account owned by the system program is created by default
pub fn init_rent_vault(ctx: Context<InitRentVault>, fund_lamports: u64) -> Result<()> {
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.rent_vault.to_account_info(),
            },
        ),
        fund_lamports,
    )?;
    Ok(())
}

>>> program-examples/basics/pda-rent-payer/anchor/programs/anchor-program-example/src/instructions/mod.rs
pub mod create_new_account;
pub mod init_rent_vault;

pub use create_new_account::*;
pub use init_rent_vault::*;

>>> program-examples/basics/pda-rent-payer/anchor/programs/anchor-program-example/src/instructions/create_new_account.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};

#[derive(Accounts)]
pub struct CreateNewAccount<'info> {
    #[account(mut)]
    new_account: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"rent_vault",
        ],
        bump,
    )]
    rent_vault: SystemAccount<'info>,
    system_program: Program<'info, System>,
}

pub fn create_new_account(ctx: Context<CreateNewAccount>) -> Result<()> {
    // PDA signer seeds
    let signer_seeds: &[&[&[u8]]] = &[&[b"rent_vault", &[ctx.bumps.rent_vault]]];

    // The minimum lamports for rent exemption
    let lamports = (Rent::get()?).minimum_balance(0);

    // Create the new account, transferring lamports from the rent vault to the new account
    create_account(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            CreateAccount {
                from: ctx.accounts.rent_vault.to_account_info(), // From pubkey
                to: ctx.accounts.new_account.to_account_info(),  // To pubkey
            },
        )
        .with_signer(signer_seeds),
        lamports,                           // Lamports
        0,                                  // Space
        &ctx.accounts.system_program.key(), // Owner Program
    )?;
    Ok(())
}

>>> program-examples/tokens/token-2022/metadata/anchor/programs/metadata/src/lib.rs
use anchor_lang::prelude::*;

use instructions::*;
mod instructions;

declare_id!("BJHEDXSQfD9kBFvhw8ZCGmPFRihzvbMoxoHUKpXdpn4D");

#[program]
pub mod metadata {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, args: TokenMetadataArgs) -> Result<()> {
        process_initialize(ctx, args)
    }

    pub fn update_field(ctx: Context<UpdateField>, args: UpdateFieldArgs) -> Result<()> {
        process_update_field(ctx, args)
    }

    pub fn remove_key(ctx: Context<RemoveKey>, key: String) -> Result<()> {
        process_remove_key(ctx, key)
    }

    pub fn emit(ctx: Context<Emit>) -> Result<()> {
        process_emit(ctx)
    }

    pub fn update_authority(ctx: Context<UpdateAuthority>) -> Result<()> {
        process_update_authority(ctx)
    }
}

>>> program-examples/tokens/token-2022/metadata/anchor/programs/metadata/src/instructions/update_authority.rs
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    spl_pod::optional_keys::OptionalNonZeroPubkey, token_metadata_update_authority, Mint,
    Token2022, TokenMetadataUpdateAuthority,
};

#[derive(Accounts)]
pub struct UpdateAuthority<'info> {
    pub current_authority: Signer<'info>,
    pub new_authority: Option<UncheckedAccount<'info>>,

    #[account(
        mut,
        extensions::metadata_pointer::metadata_address = mint_account,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn process_update_authority(ctx: Context<UpdateAuthority>) -> Result<()> {
    let new_authority_key = match &ctx.accounts.new_authority {
        Some(account) => OptionalNonZeroPubkey::try_from(Some(account.key()))?,
        None => OptionalNonZeroPubkey::try_from(None)?,
    };

    // Change update authority
    token_metadata_update_authority(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataUpdateAuthority {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                metadata: ctx.accounts.mint_account.to_account_info(),
                current_authority: ctx.accounts.current_authority.to_account_info(),

                // new authority isn't actually needed as account in the CPI
                // using current_authority as a placeholder to satisfy the struct
                new_authority: ctx.accounts.current_authority.to_account_info(),
            },
        ),
        new_authority_key,
    )?;
    Ok(())
}

>>> program-examples/tokens/token-2022/metadata/anchor/programs/metadata/src/instructions/remove_key.rs
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token_interface::{Mint, Token2022};
use spl_token_metadata_interface::instruction::remove_key;

#[derive(Accounts)]
pub struct RemoveKey<'info> {
    #[account(mut)]
    pub update_authority: Signer<'info>,

    #[account(
        mut,
        extensions::metadata_pointer::metadata_address = mint_account,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

// Invoke the remove_key instruction from spl_token_metadata_interface directly
// There is not an anchor CpiContext for this instruction
pub fn process_remove_key(ctx: Context<RemoveKey>, key: String) -> Result<()> {
    invoke(
        &remove_key(
            &ctx.accounts.token_program.key(),    // token program id
            &ctx.accounts.mint_account.key(),     // "metadata" account
            &ctx.accounts.update_authority.key(), // update authority
            key,                                  // key to remove
            true, // idempotent flag, if true transaction will not fail if key does not exist
        ),
        &[
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.mint_account.to_account_info(),
            ctx.accounts.update_authority.to_account_info(),
        ],
    )?;
    Ok(())
}

>>> program-examples/tokens/token-2022/metadata/anchor/programs/metadata/src/instructions/emit.rs
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token_interface::{Mint, Token2022};
use spl_token_metadata_interface::instruction::emit;

#[derive(Accounts)]
pub struct Emit<'info> {
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
}

// Invoke the emit instruction from spl_token_metadata_interface directly
// There is not an anchor CpiContext for this instruction
pub fn process_emit(ctx: Context<Emit>) -> Result<()> {
    invoke(
        &emit(
            &ctx.accounts.token_program.key(), // token program id
            &ctx.accounts.mint_account.key(),  // "metadata" account
            None,
            None,
        ),
        &[
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.mint_account.to_account_info(),
        ],
    )?;
    Ok(())
}

>>> program-examples/tokens/token-2022/metadata/anchor/programs/metadata/src/instructions/mod.rs
pub use initialize::*;
pub mod initialize;
pub use update_field::*;
pub mod update_field;
pub use remove_key::*;
pub mod remove_key;
pub use emit::*;
pub mod emit;
pub use update_authority::*;
pub mod update_authority;

>>> program-examples/tokens/token-2022/metadata/anchor/programs/metadata/src/instructions/update_field.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{BaseStateWithExtensions, PodStateWithExtensions},
        pod::PodMint,
    },
    token_interface::{token_metadata_update_field, Mint, Token2022, TokenMetadataUpdateField},
};
use spl_token_metadata_interface::state::{Field, TokenMetadata};

#[derive(Accounts)]
pub struct UpdateField<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        extensions::metadata_pointer::metadata_address = mint_account,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn process_update_field(ctx: Context<UpdateField>, args: UpdateFieldArgs) -> Result<()> {
    let UpdateFieldArgs { field, value } = args;

    // Convert to Field type from spl_token_metadata_interface
    let field = field.to_spl_field();
    msg!("Field: {:?}, Value: {}", field, value);

    let (current_lamports, required_lamports) = {
        // Get the current state of the mint account
        let mint = &ctx.accounts.mint_account.to_account_info();
        let buffer = mint.try_borrow_data()?;
        let state = PodStateWithExtensions::<PodMint>::unpack(&buffer)?;

        // Get and update the token metadata
        let mut token_metadata = state.get_variable_len_extension::<TokenMetadata>()?;
        token_metadata.update(field.clone(), value.clone());
        msg!("Updated TokenMetadata: {:?}", token_metadata);

        // Calculate the new account length with the updated metadata
        let new_account_len =
            state.try_get_new_account_len_for_variable_len_extension(&token_metadata)?;

        // Calculate the required lamports for the new account length
        let required_lamports = Rent::get()?.minimum_balance(new_account_len);
        // Get the current lamports of the mint account
        let current_lamports = mint.lamports();

        msg!("Required lamports: {}", required_lamports);
        msg!("Current lamports: {}", current_lamports);

        (current_lamports, required_lamports)
    };

    // Transfer lamports to mint account for the additional metadata if needed
    if required_lamports > current_lamports {
        let lamport_difference = required_lamports - current_lamports;
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            lamport_difference,
        )?;
        msg!(
            "Transferring {} lamports to metadata account",
            lamport_difference
        );
    }

    // Update token metadata
    token_metadata_update_field(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataUpdateField {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                metadata: ctx.accounts.mint_account.to_account_info(),
                update_authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        field,
        value,
    )?;
    Ok(())
}

// Custom struct to implement AnchorSerialize and AnchorDeserialize
// This is required to pass the struct as an argument to the instruction
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateFieldArgs {
    /// Field to update in the metadata
    pub field: AnchorField,
    /// Value to write for the field
    pub value: String,
}

// Need to do this so the enum shows up in the IDL
#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub enum AnchorField {
    /// The name field, corresponding to `TokenMetadata.name`
    Name,
    /// The symbol field, corresponding to `TokenMetadata.symbol`
    Symbol,
    /// The uri field, corresponding to `TokenMetadata.uri`
    Uri,
    /// A custom field, whose key is given by the associated string
    Key(String),
}

// Convert AnchorField to Field from spl_token_metadata_interface
impl AnchorField {
    fn to_spl_field(&self) -> Field {
        match self {
            AnchorField::Name => Field::Name,
            AnchorField::Symbol => Field::Symbol,
            AnchorField::Uri => Field::Uri,
            AnchorField::Key(s) => Field::Key(s.clone()),
        }
    }
}

>>> program-examples/tokens/token-2022/metadata/anchor/programs/metadata/src/instructions/initialize.rs
use anchor_lang::prelude::*;
use anchor_lang::solana_program::rent::{
    DEFAULT_EXEMPTION_THRESHOLD, DEFAULT_LAMPORTS_PER_BYTE_YEAR,
};
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::token_interface::{
    token_metadata_initialize, Mint, Token2022, TokenMetadataInitialize,
};
use spl_token_metadata_interface::state::TokenMetadata;
use spl_type_length_value::variable_len_pack::VariableLenPack;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = 2,
        mint::authority = payer,
        extensions::metadata_pointer::authority = payer,
        extensions::metadata_pointer::metadata_address = mint_account,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn process_initialize(ctx: Context<Initialize>, args: TokenMetadataArgs) -> Result<()> {
    let TokenMetadataArgs { name, symbol, uri } = args;

    // Define token metadata
    let token_metadata = TokenMetadata {
        name: name.clone(),
        symbol: symbol.clone(),
        uri: uri.clone(),
        ..Default::default()
    };

    // Add 4 extra bytes for size of MetadataExtension (2 bytes for type, 2 bytes for length)
    let data_len = 4 + token_metadata.get_packed_len()?;

    // Calculate lamports required for the additional metadata
    let lamports =
        data_len as u64 * DEFAULT_LAMPORTS_PER_BYTE_YEAR * DEFAULT_EXEMPTION_THRESHOLD as u64;

    // Transfer additional lamports to mint account
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.mint_account.to_account_info(),
            },
        ),
        lamports,
    )?;

    // Initialize token metadata
    token_metadata_initialize(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataInitialize {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                metadata: ctx.accounts.mint_account.to_account_info(),
                mint_authority: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.payer.to_account_info(),
            },
        ),
        name,
        symbol,
        uri,
    )?;
    Ok(())
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct TokenMetadataArgs {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

>>> program-examples/compression/cnft-burn/anchor/programs/cnft-burn/src/lib.rs
use anchor_lang::prelude::*;

declare_id!("FcLCJkSvwQQTDfCde5LdC4DSZAqSyb2AWM9US3wF5Fp7");

#[derive(Clone)]
pub struct SPLCompression;

impl anchor_lang::Id for SPLCompression {
    fn id() -> Pubkey {
        spl_account_compression::id()
    }
}

#[program]
pub mod cnft_burn {
    use super::*;

    pub fn burn_cnft<'info>(
        ctx: Context<'_, '_, '_, 'info, BurnCnft<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        let tree_config = ctx.accounts.tree_authority.to_account_info();
        let leaf_owner = ctx.accounts.leaf_owner.to_account_info();
        let merkle_tree = ctx.accounts.merkle_tree.to_account_info();
        let log_wrapper = ctx.accounts.log_wrapper.to_account_info();
        let compression_program = ctx.accounts.compression_program.to_account_info();
        let system_program = ctx.accounts.system_program.to_account_info();

        let cnft_burn_cpi = mpl_bubblegum::instructions::BurnCpi::new(
            &ctx.accounts.bubblegum_program,
            mpl_bubblegum::instructions::BurnCpiAccounts {
                tree_config: &tree_config,
                leaf_owner: (&leaf_owner, true),
                leaf_delegate: (&leaf_owner, false),
                merkle_tree: &merkle_tree,
                log_wrapper: &log_wrapper,
                compression_program: &compression_program,
                system_program: &system_program,
            },
            mpl_bubblegum::instructions::BurnInstructionArgs {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
            },
        );

        cnft_burn_cpi.invoke_with_remaining_accounts(
            ctx.remaining_accounts
                .iter()
                .map(|account| (account, false, false))
                .collect::<Vec<_>>()
                .as_slice(),
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct BurnCnft<'info> {
    #[account(mut)]
    pub leaf_owner: Signer<'info>,
    #[account(mut)]
    #[account(
        seeds = [merkle_tree.key().as_ref()],
        bump,
        seeds::program = bubblegum_program.key()
    )]
    /// CHECK: This account is modified in the downstream program
    pub tree_authority: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account is neither written to nor read from.
    pub merkle_tree: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub log_wrapper: UncheckedAccount<'info>,
    pub compression_program: Program<'info, SPLCompression>,
    /// CHECK: This account is neither written to nor read from.
    pub bubblegum_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/state/contributor.rs
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Contributor {
    pub amount: u64,
}
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/state/mod.rs
pub mod fundraiser;
pub mod contributor;

pub use fundraiser::*;
pub use contributor::*;
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/state/fundraiser.rs
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Fundraiser {
    pub maker: Pubkey,
    pub mint_to_raise: Pubkey,
    pub amount_to_raise: u64,
    pub current_amount: u64,
    pub time_started: i64,
    pub duration: u16,
    pub bump: u8,
}
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/lib.rs
use anchor_lang::prelude::*;

declare_id!("Eoiuq1dXvHxh6dLx3wh9gj8kSAUpga11krTrbfF5XYsC");

mod state;
mod instructions;
mod error;
mod constants;

use instructions::*;
use error::*;
pub use constants::*;

#[program]
pub mod fundraiser {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, amount: u64, duration: u16) -> Result<()> {

        ctx.accounts.initialize(amount, duration, &ctx.bumps)?;

        Ok(())
    }

    pub fn contribute(ctx: Context<Contribute>, amount: u64) -> Result<()> {

        ctx.accounts.contribute(amount)?;

        Ok(())
    }

    pub fn check_contributions(ctx: Context<CheckContributions>) -> Result<()> {

        ctx.accounts.check_contributions()?;

        Ok(())
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {

        ctx.accounts.refund()?;

        Ok(())
    }
}

>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/error.rs
use anchor_lang::error_code;

#[error_code]
pub enum FundraiserError {
    #[msg("The amount to raise has not been met")]
    TargetNotMet,
    #[msg("The amount to raise has been achieved")]
    TargetMet,
    #[msg("The contribution is too big")]
    ContributionTooBig,
    #[msg("The contribution is too small")]
    ContributionTooSmall,
    #[msg("The maximum amount to contribute has been reached")]
    MaximumContributionsReached,
    #[msg("The fundraiser has not ended yet")]
    FundraiserNotEnded,
    #[msg("The fundraiser has ended")]
    FundraiserEnded,
    #[msg("Invalid total amount. i should be bigger than 3")]
    InvalidAmount
}
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/constants.rs
pub const ANCHOR_DISCRIMINATOR: usize = 8;
pub const MIN_AMOUNT_TO_RAISE: u64 = 3;
pub const SECONDS_TO_DAYS: i64 = 86400;
pub const MAX_CONTRIBUTION_PERCENTAGE: u64 = 10;
pub const PERCENTAGE_SCALER: u64 = 100;
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/instructions/contribute.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{
    Mint, 
    transfer, 
    Token, 
    TokenAccount, 
    Transfer
};

use crate::{
    state::{
        Contributor, 
        Fundraiser
    }, FundraiserError, 
    ANCHOR_DISCRIMINATOR, 
    MAX_CONTRIBUTION_PERCENTAGE, 
    PERCENTAGE_SCALER, SECONDS_TO_DAYS
};

#[derive(Accounts)]
pub struct Contribute<'info> {
    #[account(mut)]
    pub contributor: Signer<'info>,
    pub mint_to_raise: Account<'info, Mint>,
    #[account(
        mut,
        has_one = mint_to_raise,
        seeds = [b"fundraiser".as_ref(), fundraiser.maker.as_ref()],
        bump = fundraiser.bump,
    )]
    pub fundraiser: Account<'info, Fundraiser>,
    #[account(
        init_if_needed,
        payer = contributor,
        seeds = [b"contributor", fundraiser.key().as_ref(), contributor.key().as_ref()],
        bump,
        space = ANCHOR_DISCRIMINATOR + Contributor::INIT_SPACE,
    )]
    pub contributor_account: Account<'info, Contributor>,
    #[account(
        mut,
        associated_token::mint = mint_to_raise,
        associated_token::authority = contributor
    )]
    pub contributor_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = fundraiser.mint_to_raise,
        associated_token::authority = fundraiser
    )]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Contribute<'info> {
    pub fn contribute(&mut self, amount: u64) -> Result<()> {

        // Check if the amount to contribute meets the minimum amount required
        require!(
            amount >= 1_u64.pow(self.mint_to_raise.decimals as u32), 
            FundraiserError::ContributionTooSmall
        );

        // Check if the amount to contribute is less than the maximum allowed contribution
        require!(
            amount <= (self.fundraiser.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER, 
            FundraiserError::ContributionTooBig
        );

        // Check if the fundraising duration has been reached
        let current_time = Clock::get()?.unix_timestamp;
        require!(
            self.fundraiser.duration <= ((current_time - self.fundraiser.time_started) / SECONDS_TO_DAYS) as u16,
            crate::FundraiserError::FundraiserEnded
        );

        // Check if the maximum contributions per contributor have been reached
        require!(
            (self.contributor_account.amount <= (self.fundraiser.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER)
                && (self.contributor_account.amount + amount <= (self.fundraiser.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER),
            FundraiserError::MaximumContributionsReached
        );

        // Transfer the funds to the vault
        // CPI to the token program to transfer the funds
        let cpi_program = self.token_program.to_account_info();

        // Transfer the funds from the contributor to the vault
        let cpi_accounts = Transfer {
            from: self.contributor_ata.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.contributor.to_account_info(),
        };

        // Crete a CPI context
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        // Transfer the funds from the contributor to the vault
        transfer(cpi_ctx, amount)?;

        // Update the fundraiser and contributor accounts with the new amounts
        self.fundraiser.current_amount += amount;

        self.contributor_account.amount += amount;

        Ok(())
    }
}
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/instructions/checker.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token::{
        transfer, 
        Mint, 
        Token, 
        TokenAccount, 
        Transfer
    }
};

use crate::{
    state::Fundraiser, 
    FundraiserError
};

#[derive(Accounts)]
pub struct CheckContributions<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    pub mint_to_raise: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"fundraiser".as_ref(), maker.key().as_ref()],
        bump = fundraiser.bump,
        close = maker,
    )]
    pub fundraiser: Account<'info, Fundraiser>,
    #[account(
        mut,
        associated_token::mint = mint_to_raise,
        associated_token::authority = fundraiser,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = mint_to_raise,
        associated_token::authority = maker,
    )]
    pub maker_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> CheckContributions<'info> {
    pub fn check_contributions(&self) -> Result<()> {
        
        // Check if the target amount has been met
        require!(
            self.vault.amount >= self.fundraiser.amount_to_raise,
            FundraiserError::TargetNotMet
        );

        // Transfer the funds to the maker
        // CPI to the token program to transfer the funds
        let cpi_program = self.token_program.to_account_info();

        // Transfer the funds from the vault to the maker
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.maker_ata.to_account_info(),
            authority: self.fundraiser.to_account_info(),
        };

        // Signer seeds to sign the CPI on behalf of the fundraiser account
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"fundraiser".as_ref(),
            self.maker.to_account_info().key.as_ref(),
            &[self.fundraiser.bump],
        ]];

        // CPI context with signer since the fundraiser account is a PDA
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &signer_seeds);

        // Transfer the funds from the vault to the maker
        transfer(cpi_ctx, self.vault.amount)?;

        Ok(())
    }
}
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/instructions/mod.rs
pub mod initialize;
pub mod contribute;
pub mod checker;
pub mod refund;

pub use initialize::*;
pub use contribute::*;
pub use checker::*;
pub use refund::*;
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/instructions/refund.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{
    transfer, 
    Mint, 
    Token, 
    TokenAccount, 
    Transfer
};

use crate::{
    state::{
        Contributor, 
        Fundraiser
    }, 
    SECONDS_TO_DAYS
};

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub contributor: Signer<'info>,
    pub maker: SystemAccount<'info>,
    pub mint_to_raise: Account<'info, Mint>,
    #[account(
        mut,
        has_one = mint_to_raise,
        seeds = [b"fundraiser", maker.key().as_ref()],
        bump = fundraiser.bump,
    )]
    pub fundraiser: Account<'info, Fundraiser>,
    #[account(
        mut,
        seeds = [b"contributor", fundraiser.key().as_ref(), contributor.key().as_ref()],
        bump,
        close = contributor,
    )]
    pub contributor_account: Account<'info, Contributor>,
    #[account(
        mut,
        associated_token::mint = mint_to_raise,
        associated_token::authority = contributor
    )]
    pub contributor_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_to_raise,
        associated_token::authority = fundraiser
    )]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Refund<'info> {
    pub fn refund(&mut self) -> Result<()> {

        // Check if the fundraising duration has been reached
        let current_time = Clock::get()?.unix_timestamp;
 
        require!(
            self.fundraiser.duration >= ((current_time - self.fundraiser.time_started) / SECONDS_TO_DAYS) as u16,
            crate::FundraiserError::FundraiserNotEnded
        );

        require!(
            self.vault.amount < self.fundraiser.amount_to_raise,
            crate::FundraiserError::TargetMet
        );

        // Transfer the funds back to the contributor
        // CPI to the token program to transfer the funds
        let cpi_program = self.token_program.to_account_info();

        // Transfer the funds from the vault to the contributor
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.contributor_ata.to_account_info(),
            authority: self.fundraiser.to_account_info(),
        };

        // Signer seeds to sign the CPI on behalf of the fundraiser account
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"fundraiser".as_ref(),
            self.maker.to_account_info().key.as_ref(),
            &[self.fundraiser.bump],
        ]];

        // CPI context with signer since the fundraiser account is a PDA
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, &signer_seeds);

        // Transfer the funds from the vault to the contributor
        transfer(cpi_ctx, self.contributor_account.amount)?;

        // Update the fundraiser state by reducing the amount contributed
        self.fundraiser.current_amount -= self.contributor_account.amount;

        Ok(())
    }
}
>>> program-examples/tokens/token-fundraiser/anchor/programs/fundraiser/src/instructions/initialize.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token::{
        Mint, 
        Token, 
        TokenAccount
    }
};

use crate::{
    state::Fundraiser, FundraiserError, ANCHOR_DISCRIMINATOR, MIN_AMOUNT_TO_RAISE
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    pub mint_to_raise: Account<'info, Mint>,
    #[account(
        init,
        payer = maker,
        seeds = [b"fundraiser", maker.key().as_ref()],
        bump,
        space = ANCHOR_DISCRIMINATOR + Fundraiser::INIT_SPACE,
    )]
    pub fundraiser: Account<'info, Fundraiser>,
    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_to_raise,
        associated_token::authority = fundraiser,
    )]
    pub vault: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, amount: u64, duration: u16, bumps: &InitializeBumps) -> Result<()> {

        // Check if the amount to raise meets the minimum amount required
        require!(
            amount >= MIN_AMOUNT_TO_RAISE.pow(self.mint_to_raise.decimals as u32),
            FundraiserError::InvalidAmount
        );

        // Initialize the fundraiser account
        self.fundraiser.set_inner(Fundraiser {
            maker: self.maker.key(),
            mint_to_raise: self.mint_to_raise.key(),
            amount_to_raise: amount,
            current_amount: 0,
            time_started: Clock::get()?.unix_timestamp,
            duration,
            bump: bumps.fundraiser
        });
        
        Ok(())
    }
}
>>> program-examples/tokens/token-2022/mint-close-authority/anchor/programs/mint-close-authority/src/lib.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{close_account, CloseAccount},
    token_interface::{
        spl_pod::optional_keys::OptionalNonZeroPubkey,
        spl_token_2022::{
            extension::{
                mint_close_authority::MintCloseAuthority, BaseStateWithExtensions,
                StateWithExtensions,
            },
            state::Mint as MintState,
        },
        Mint, Token2022,
    },
};
declare_id!("AcfQLsYKuzprcCNH1n96pKKgAbAnZchwpbr3gbVN742n");

#[program]
pub mod mint_close_authority {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.check_mint_data()?;
        Ok(())
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        // cpi to token extensions programs to close mint account
        // alternatively, this can also be done in the client
        close_account(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.mint_account.to_account_info(),
                destination: ctx.accounts.authority.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ))?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = 2,
        mint::authority = payer,
        extensions::close_authority::authority = payer,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

// helper to check mint data, and demonstrate how to read mint extension data within a program
impl<'info> Initialize<'info> {
    pub fn check_mint_data(&self) -> Result<()> {
        let mint = &self.mint_account.to_account_info();
        let mint_data = mint.data.borrow();
        let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
        let extension_data = mint_with_extension.get_extension::<MintCloseAuthority>()?;

        assert_eq!(
            extension_data.close_authority,
            OptionalNonZeroPubkey::try_from(Some(self.payer.key()))?
        );

        msg!("{:?}", extension_data);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        extensions::close_authority::authority = authority,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
}

>>> program-examples/tokens/token-2022/permanent-delegate/anchor/programs/permanent-delegate/src/lib.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::extension::permanent_delegate::PermanentDelegate,
    token_interface::{
        spl_pod::optional_keys::OptionalNonZeroPubkey,
        spl_token_2022::{
            extension::{BaseStateWithExtensions, StateWithExtensions},
            state::Mint as MintState,
        },
        Mint, Token2022,
    },
};

declare_id!("A9rxKS84ZoJVyeTfQbCEfxME2vvAM4uwSMjkmhR5XWb1");

#[program]
pub mod permanent_delegate {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.check_mint_data()?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = 2,
        mint::authority = payer,
        extensions::permanent_delegate::delegate = payer,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

// helper to check mint data, and demonstrate how to read mint extension data within a program
impl<'info> Initialize<'info> {
    pub fn check_mint_data(&self) -> Result<()> {
        let mint = &self.mint_account.to_account_info();
        let mint_data = mint.data.borrow();
        let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
        let extension_data = mint_with_extension.get_extension::<PermanentDelegate>()?;

        assert_eq!(
            extension_data.delegate,
            OptionalNonZeroPubkey::try_from(Some(self.payer.key()))?
        );

        msg!("{:?}", extension_data);
        Ok(())
    }
}

>>> program-examples/compression/cutils/anchor/programs/cutils/src/actions/mint.rs
use crate::*;
use mpl_bubblegum::state::{
    metaplex_adapter::{Collection, Creator, MetadataArgs, TokenProgramVersion, TokenStandard},
    metaplex_anchor::{MplTokenMetadata, TokenMetadata},
    TreeConfig, COLLECTION_CPI_PREFIX,
};

#[derive(Accounts)]
#[instruction(params: MintParams)]
pub struct Mint<'info> {
    // #[account(
    //     init,
    //     seeds = [
    //         SEED_DATA,
    //         data.tree,
    //         data.tree_nonce
    //         // assetId directly?
    //     ],
    //     bump,
    //     payer = payer,
    //     space = Data::LEN,
    // )]
    // pub data: Account<'info, Data>,
    pub payer: Signer<'info>,

    // Bubblegum cNFT stuff MintToCollectionV1
    #[account(
        mut,
        seeds = [merkle_tree.key().as_ref()],
        seeds::program = bubblegum_program.key(),
        bump,
    )]
    pub tree_authority: Box<Account<'info, TreeConfig>>,

    /// CHECK: This account is neither written to nor read from.
    pub leaf_owner: AccountInfo<'info>,

    /// CHECK: This account is neither written to nor read from.
    pub leaf_delegate: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_tree: UncheckedAccount<'info>,

    pub tree_delegate: Signer<'info>,

    pub collection_authority: Signer<'info>,

    /// CHECK: Optional collection authority record PDA.
    /// If there is no collecton authority record PDA then
    /// this must be the Bubblegum program address.
    pub collection_authority_record_pda: UncheckedAccount<'info>,

    /// CHECK: This account is checked in the instruction
    pub collection_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub collection_metadata: Box<Account<'info, TokenMetadata>>,

    /// CHECK: This account is checked in the instruction
    pub edition_account: UncheckedAccount<'info>,

    /// CHECK: This is just used as a signing PDA.
    #[account(
        seeds = [COLLECTION_CPI_PREFIX.as_ref()],
        seeds::program = bubblegum_program.key(),
        bump,
    )]
    pub bubblegum_signer: UncheckedAccount<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub token_metadata_program: Program<'info, MplTokenMetadata>,
    pub bubblegum_program: Program<'info, MplBubblegum>,
    pub system_program: Program<'info, System>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct MintParams {
    uri: String,
}

impl Mint<'_> {
    pub fn validate(&self, _ctx: &Context<Self>, _params: &MintParams) -> Result<()> {
        Ok(())
    }

    pub fn actuate<'info>(
        ctx: Context<'_, '_, '_, 'info, Mint<'info>>,
        params: MintParams,
    ) -> Result<()> {
        mpl_bubblegum::cpi::mint_to_collection_v1(
            CpiContext::new(
                ctx.accounts.bubblegum_program.to_account_info(),
                mpl_bubblegum::cpi::accounts::MintToCollectionV1 {
                    tree_authority: ctx.accounts.tree_authority.to_account_info(),
                    leaf_owner: ctx.accounts.leaf_owner.to_account_info(),
                    leaf_delegate: ctx.accounts.leaf_delegate.to_account_info(),
                    merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
                    payer: ctx.accounts.payer.to_account_info(),
                    tree_delegate: ctx.accounts.tree_delegate.to_account_info(),
                    collection_authority: ctx.accounts.collection_authority.to_account_info(),
                    collection_authority_record_pda: ctx
                        .accounts
                        .collection_authority_record_pda
                        .to_account_info(),
                    collection_mint: ctx.accounts.collection_mint.to_account_info(),
                    collection_metadata: ctx.accounts.collection_metadata.to_account_info(),
                    edition_account: ctx.accounts.edition_account.to_account_info(),
                    bubblegum_signer: ctx.accounts.bubblegum_signer.to_account_info(),
                    log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
                    compression_program: ctx.accounts.compression_program.to_account_info(),
                    token_metadata_program: ctx.accounts.token_metadata_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            MetadataArgs {
                name: "BURGER".to_string(),
                symbol: "BURG".to_string(),
                uri: params.uri,
                creators: vec![Creator {
                    address: ctx.accounts.collection_authority.key(),
                    verified: false,
                    share: 100,
                }],
                seller_fee_basis_points: 0,
                primary_sale_happened: false,
                is_mutable: false,
                edition_nonce: Some(0),
                uses: None,
                collection: Some(Collection {
                    verified: false,
                    key: ctx.accounts.collection_mint.key(),
                }),
                token_program_version: TokenProgramVersion::Original,
                token_standard: Some(TokenStandard::NonFungible),
            },
        )?;

        Ok(())
    }
}

>>> program-examples/tokens/token-2022/memo-transfer/anchor/programs/memo-transfer/src/lib.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    token_2022::{
        initialize_account3,
        spl_token_2022::{extension::ExtensionType, pod::PodAccount},
        InitializeAccount3,
    },
    token_interface::{
        memo_transfer_disable, memo_transfer_initialize, MemoTransfer, Mint, Token2022,
        TokenAccount,
    },
};

declare_id!("5BQyC7y2Pc283woThq11uZRqsgcRbBRLKz4yQ8BJadi2");

#[program]
pub mod memo_transfer {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        // Calculate space required for token and extension data
        let token_account_size =
            ExtensionType::try_calculate_account_len::<PodAccount>(&[ExtensionType::MemoTransfer])?;

        // Calculate minimum lamports required for size of token account with extensions
        let lamports = (Rent::get()?).minimum_balance(token_account_size);

        // Invoke System Program to create new account with space for token account and extension data
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                },
            ),
            lamports,                          // Lamports
            token_account_size as u64,         // Space
            &ctx.accounts.token_program.key(), // Owner Program
        )?;

        // Initialize the standard token account data
        initialize_account3(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            InitializeAccount3 {
                account: ctx.accounts.token_account.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        ))?;

        // Initialize the memo transfer extension
        // This instruction must come after the token account initialization
        memo_transfer_initialize(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MemoTransfer {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                account: ctx.accounts.token_account.to_account_info(),
                owner: ctx.accounts.payer.to_account_info(),
            },
        ))?;
        Ok(())
    }

    pub fn disable(ctx: Context<Disable>) -> Result<()> {
        memo_transfer_disable(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MemoTransfer {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                account: ctx.accounts.token_account.to_account_info(),
                owner: ctx.accounts.owner.to_account_info(),
            },
        ))?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub token_account: Signer<'info>,
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Disable<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        token::authority = owner,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token2022>,
}

>>> program-examples/compression/cutils/anchor/programs/cutils/src/actions/verify.rs
use crate::*;
use mpl_bubblegum::state::leaf_schema::LeafSchema;
use mpl_bubblegum::utils::get_asset_id;
use spl_account_compression::program::SplAccountCompression;

#[derive(Accounts)]
#[instruction(params: VerifyParams)]
pub struct Verify<'info> {
    pub leaf_owner: Signer<'info>,

    /// CHECK: This account is neither written to nor read from.
    pub leaf_delegate: AccountInfo<'info>,

    /// CHECK: unsafe
    pub merkle_tree: UncheckedAccount<'info>,

    pub compression_program: Program<'info, SplAccountCompression>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct VerifyParams {
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
}

impl Verify<'_> {
    pub fn validate(&self, _ctx: &Context<Self>, _params: &VerifyParams) -> Result<()> {
        Ok(())
    }

    pub fn actuate<'info>(
        ctx: Context<'_, '_, '_, 'info, Verify<'info>>,
        params: &VerifyParams,
    ) -> Result<()> {
        let asset_id = get_asset_id(&ctx.accounts.merkle_tree.key(), params.nonce);
        let leaf = LeafSchema::new_v0(
            asset_id,
            ctx.accounts.leaf_owner.key(),
            ctx.accounts.leaf_delegate.key(),
            params.nonce,
            params.data_hash,
            params.creator_hash,
        );

        let cpi_ctx = CpiContext::new(
            ctx.accounts.compression_program.to_account_info(),
            spl_account_compression::cpi::accounts::VerifyLeaf {
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
            },
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());

        spl_account_compression::cpi::verify_leaf(
            cpi_ctx,
            params.root,
            leaf.to_node(),
            params.index,
        )?;

        Ok(())
    }
}

>>> program-examples/compression/cutils/anchor/programs/cutils/src/actions/mod.rs
pub mod mint;
pub use mint::*;

pub mod verify;
pub use verify::*;

>>> program-examples/compression/cutils/anchor/programs/cutils/src/state/data.rs
use crate::*;

pub const SEED_DATA: &[u8] = b"DATA";

#[account]
#[derive(Default, Debug)]
pub struct Data {
    /// The bump, used for PDA validation.
    pub bump: u8,
    pub tree: Pubkey,
    pub tree_nonce: u64,
}

impl Data {
    pub const LEN: usize = 8 + 1 + 32 + 8;

    pub fn new(bump: u8, tree: Pubkey, tree_nonce: u64) -> Self {
        Self {
            bump,
            tree,
            tree_nonce,
        }
    }
}

>>> program-examples/compression/cutils/anchor/programs/cutils/src/state/mod.rs
pub mod data;
pub use data::*;

>>> program-examples/compression/cutils/anchor/programs/cutils/src/lib.rs
#![allow(clippy::result_large_err)]

pub mod actions;
pub use actions::*;

pub mod state;
pub use state::*;

use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;
use spl_account_compression::{program::SplAccountCompression, Noop};

#[derive(Clone)]
pub struct MplBubblegum;

impl anchor_lang::Id for MplBubblegum {
    fn id() -> Pubkey {
        mpl_bubblegum::id()
    }
}

declare_id!("burZc1SfqbrAP35XG63YZZ82C9Zd22QUwhCXoEUZWNF");

#[program]
pub mod cutils {
    use super::*;

    #[access_control(ctx.accounts.validate(&ctx, &params))]
    pub fn mint<'info>(
        ctx: Context<'_, '_, '_, 'info, Mint<'info>>,
        params: MintParams,
    ) -> Result<()> {
        Mint::actuate(ctx, params)
    }

    #[access_control(ctx.accounts.validate(&ctx, &params))]
    pub fn verify<'info>(
        ctx: Context<'_, '_, '_, 'info, Verify<'info>>,
        params: VerifyParams,
    ) -> Result<()> {
        Verify::actuate(ctx, &params)
    }
}

>>> program-examples/tokens/token-2022/immutable-owner/anchor/programs/immutable-owner/src/lib.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    token_2022::{
        initialize_account3,
        spl_token_2022::{extension::ExtensionType, pod::PodAccount},
        InitializeAccount3,
    },
    token_interface::{immutable_owner_initialize, ImmutableOwnerInitialize, Mint, Token2022},
};

declare_id!("6g5URpqqurW8RbKjuGeRCVZBKky3J4kYcLeotQ6vj6UT");

#[program]
pub mod immutable_owner {
    use super::*;

    // There is currently not an anchor constraint to automatically initialize the ImmutableOwner extension
    // We can manually create and initialize the token account via CPIs in the instruction handler
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        // Calculate space required for token and extension data
        let token_account_size = ExtensionType::try_calculate_account_len::<PodAccount>(&[
            ExtensionType::ImmutableOwner,
        ])?;

        // Calculate minimum lamports required for size of token account with extensions
        let lamports = (Rent::get()?).minimum_balance(token_account_size);

        // Invoke System Program to create new account with space for token account and extension data
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                },
            ),
            lamports,                          // Lamports
            token_account_size as u64,         // Space
            &ctx.accounts.token_program.key(), // Owner Program
        )?;

        // Initialize the token account with the immutable owner extension
        immutable_owner_initialize(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            ImmutableOwnerInitialize {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                token_account: ctx.accounts.token_account.to_account_info(),
            },
        ))?;

        // Initialize the standard token account data
        initialize_account3(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            InitializeAccount3 {
                account: ctx.accounts.token_account.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        ))?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub token_account: Signer<'info>,
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/external-delegate-token-master/anchor/programs/external-delegate-token-master/src/lib.rs
use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{Token, TokenAccount, Transfer};
use solana_program::secp256k1_recover::secp256k1_recover;
use sha3::{Digest, Keccak256};

declare_id!("FYPkt5VWMvtyWZDMGCwoKFkE3wXTzphicTpnNGuHWVbD");

#[program]
pub mod external_delegate_token_master {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.authority = ctx.accounts.authority.key();
        user_account.ethereum_address = [0; 20];
        Ok(())
    }

    pub fn set_ethereum_address(ctx: Context<SetEthereumAddress>, ethereum_address: [u8; 20]) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.ethereum_address = ethereum_address;
        Ok(())
    }

    pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64, signature: [u8; 65], message: [u8; 32]) -> Result<()> {
        let user_account = &ctx.accounts.user_account;

        if !verify_ethereum_signature(&user_account.ethereum_address, &message, &signature) {
            return Err(ErrorCode::InvalidSignature.into());
        }

        // Transfer tokens
        let transfer_instruction = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.user_pda.to_account_info(),
        };

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_instruction,
                &[&[
                    user_account.key().as_ref(),
                    &[ctx.bumps.user_pda],
                ]],
            ),
            amount,
        )?;

        Ok(())
    }

    pub fn authority_transfer(ctx: Context<AuthorityTransfer>, amount: u64) -> Result<()> {
        // Transfer tokens
        let transfer_instruction = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.user_pda.to_account_info(),
        };

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_instruction,
                &[&[
                    ctx.accounts.user_account.key().as_ref(),
                    &[ctx.bumps.user_pda],
                ]],
            ),
            amount,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 20)] // Ensure this is only for user_account
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub authority: Signer<'info>, // This should remain as a signer
    pub system_program: Program<'info, System>, // Required for initialization
}

#[derive(Accounts)]
pub struct SetEthereumAddress<'info> {
    #[account(mut, has_one = authority)]
    pub user_account: Account<'info, UserAccount>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(has_one = authority)]
    pub user_account: Account<'info, UserAccount>,
    pub authority: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [user_account.key().as_ref()],
        bump,
    )]
    pub user_pda: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AuthorityTransfer<'info> {
    #[account(has_one = authority)]
    pub user_account: Account<'info, UserAccount>,
    pub authority: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    #[account(
        seeds = [user_account.key().as_ref()],
        bump,
    )]
    pub user_pda: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct UserAccount {
    pub authority: Pubkey,
    pub ethereum_address: [u8; 20],
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Ethereum signature")]
    InvalidSignature,
}

fn verify_ethereum_signature(ethereum_address: &[u8; 20], message: &[u8; 32], signature: &[u8; 65]) -> bool {
    let recovery_id = signature[64];
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&signature[..64]);

    if let Ok(pubkey) = secp256k1_recover(message, recovery_id, &sig) {
        let pubkey_bytes = pubkey.to_bytes();
        let mut recovered_address = [0u8; 20];
        recovered_address.copy_from_slice(&keccak256(&pubkey_bytes[1..])[12..]);
        recovered_address == *ethereum_address
    } else {
        false
    }
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

>>> program-examples/basics/hello-solana/anchor/programs/hello-solana/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

declare_id!("2phbC62wekpw95XuBk4i1KX4uA8zBUWmYbiTMhicSuBV");

#[program]
pub mod hello_solana {
    use super::*;

    pub fn hello(_ctx: Context<Hello>) -> Result<()> {
        msg!("Hello, Solana!");

        msg!("Our program's Program ID: {}", &id());

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Hello {}

>>> program-examples/tokens/token-2022/basics/anchor/programs/basics/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    self, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

declare_id!("6qNqxkRF791FXFeQwqYQLEzAbGiqDULC5SSHVsfRoG89");

#[program]
pub mod anchor {

    use super::*;

    pub fn create_token(_ctx: Context<CreateToken>, _token_name: String) -> Result<()> {
        msg!("Create Token");
        Ok(())
    }
    pub fn create_token_account(_ctx: Context<CreateTokenAccount>) -> Result<()> {
        msg!("Create Token Account");
        Ok(())
    }
    pub fn create_associated_token_account(
        _ctx: Context<CreateAssociatedTokenAccount>,
    ) -> Result<()> {
        msg!("Create Associated Token Account");
        Ok(())
    }
    pub fn transfer_token(ctx: Context<TransferToken>, amount: u64) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.from.to_account_info().clone(),
            mint: ctx.accounts.mint.to_account_info().clone(),
            to: ctx.accounts.to_ata.to_account_info().clone(),
            authority: ctx.accounts.signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
        token_interface::transfer_checked(cpi_context, amount, ctx.accounts.mint.decimals)?;
        msg!("Transfer Token");
        Ok(())
    }
    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info().clone(),
            to: ctx.accounts.receiver.to_account_info().clone(),
            authority: ctx.accounts.signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
        token_interface::mint_to(cpi_context, amount)?;
        msg!("Mint Token");
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(token_name: String)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 6,
        mint::authority = signer.key(),
        seeds = [b"token-2022-token", signer.key().as_ref(), token_name.as_bytes()],
        bump,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct CreateTokenAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        token::mint = mint,
        token::authority = signer,
        payer = signer,
        seeds = [b"token-2022-token-account", signer.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct CreateAssociatedTokenAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        associated_token::mint = mint,
        payer = signer,
        associated_token::authority = signer,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]

pub struct TransferToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub from: InterfaceAccount<'info, TokenAccount>,
    pub to: SystemAccount<'info>,
    #[account(
        init,
        associated_token::mint = mint,
        payer = signer,
        associated_token::authority = to
    )]
    pub to_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub receiver: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

>>> program-examples/basics/favorites/anchor/programs/favorites/src/lib.rs
use anchor_lang::prelude::*;
// Our program's address!
// This matches the key in the target/deploy directory
declare_id!("ww9C83noARSQVBnqmCUmaVdbJjmiwcV9j2LkXYMoUCV");

// Anchor programs always use 8 bits for the discriminator
pub const ANCHOR_DISCRIMINATOR_SIZE: usize = 8;

// Our Solana program! 
#[program]
pub mod favorites {
    use super::*;

    // Our instruction handler! It sets the user's favorite number and color
    pub fn set_favorites(context: Context<SetFavorites>, number: u64, color: String, hobbies: Vec<String>) -> Result<()> {
        msg!("Greetings from {}", context.program_id);
        let user_public_key = context.accounts.user.key();
        msg!(
            "User {user_public_key}'s favorite number is {number}, favorite color is: {color}, and their hobbies are {hobbies:?}",
        );

        context.accounts.favorites.set_inner(Favorites {
            number,
            color,
            hobbies
        });
        Ok(())
    }

    // We can also add a get_favorites instruction handler to return the user's favorite number and color
}

// What we will put inside the Favorites PDA
#[account]
#[derive(InitSpace)]
pub struct Favorites {
    pub number: u64,

    #[max_len(50)]
    pub color: String,

    #[max_len(5, 50)]
    pub hobbies: Vec<String>
}
// When people call the set_favorites instruction, they will need to provide the accounts that will be modifed. This keeps Solana fast!
#[derive(Accounts)]
pub struct SetFavorites<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed, 
        payer = user, 
        space = ANCHOR_DISCRIMINATOR_SIZE + Favorites::INIT_SPACE, 
        seeds=[b"favorites", user.key().as_ref()],
        bump
    )]
    pub favorites: Account<'info, Favorites>,

    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-2022/cpi-guard/anchor/programs/cpi-guard/src/lib.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::{transfer_checked, TransferChecked},
    token_interface::{Mint, Token2022, TokenAccount},
};

// Note that you cannot initialize or update the CpiGuard extension through a CPI
// https://github.com/solana-labs/solana-program-library/blob/6968859e2ee0a1764da572de340cdb58e2b4586f/token/program-2022/src/extension/cpi_guard/processor.rs#L44-L46
declare_id!("6tU3MEowU6oxxeDZLSxEwzcEZsZrhBJsfUR6xECvShid");

#[program]
pub mod cpi_guard {
    use super::*;

    pub fn cpi_transfer(ctx: Context<CpiTransfer>) -> Result<()> {
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.sender_token_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    to: ctx.accounts.recipient_token_account.to_account_info(),
                    authority: ctx.accounts.sender.to_account_info(),
                },
            ),
            1,
            ctx.accounts.mint_account.decimals,
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CpiTransfer<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        token::mint = mint_account
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = sender,
        seeds = [b"pda"],
        bump,
        token::mint = mint_account,
        token::authority = recipient_token_account,
        token::token_program = token_program
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/errors.rs
use anchor_lang::error_code;

#[error_code]
pub enum GameErrorCode {
    #[msg("Not enough energy")]
    NotEnoughEnergy,
    #[msg("Wrong Authority")]
    WrongAuthority,
}

#[error_code]
pub enum ProgramErrorCode {
    #[msg("Invalid Mint account space")]
    InvalidMintAccountSpace,
    #[msg("Cant initialize metadata_pointer")]
    CantInitializeMetadataPointer,
}

>>> program-examples/basics/processing-instructions/anchor/programs/processing-instructions/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

declare_id!("DgoL5J44aspizyUs9fcnpGEUJjWTLJRCfx8eYtUMYczf");

#[program]
pub mod processing_instructions {
    use super::*;

    // With Anchor, we just put instruction data in the function signature!
    //
    pub fn go_to_park(_ctx: Context<Park>, name: String, height: u32) -> Result<()> {
        msg!("Welcome to the park, {}!", name);
        if height > 5 {
            msg!("You are tall enough to ride this ride. Congratulations.");
        } else {
            msg!("You are NOT tall enough to ride this ride. Sorry mate.");
        };

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Park {}

>>> program-examples/basics/program-derived-addresses/anchor/programs/anchor-program-example/src/state/mod.rs
pub mod page_visits;

pub use page_visits::*;

>>> program-examples/basics/program-derived-addresses/anchor/programs/anchor-program-example/src/state/page_visits.rs
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)] // automatically calculate the space required for the struct
pub struct PageVisits {
    pub page_visits: u32,
    pub bump: u8,
}

impl PageVisits {
    pub const SEED_PREFIX: &'static [u8; 11] = b"page_visits";

    pub fn increment(&mut self) {
        self.page_visits = self.page_visits.checked_add(1).unwrap();
    }
}

>>> program-examples/basics/program-derived-addresses/anchor/programs/anchor-program-example/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

use instructions::*;

pub mod instructions;
pub mod state;

declare_id!("oCCQRZyAbVxujyd8m57MPmDzZDmy2FoKW4ULS7KofCE");

#[program]
pub mod anchor_program_example {
    use super::*;

    pub fn create_page_visits(ctx: Context<CreatePageVisits>) -> Result<()> {
        create::create_page_visits(ctx)
    }

    pub fn increment_page_visits(ctx: Context<IncrementPageVisits>) -> Result<()> {
        increment::increment_page_visits(ctx)
    }
}

>>> program-examples/basics/rent/anchor/programs/rent-example/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("ED6f4gweAE7hWPQPXMt4kWxzDJne8VQEm9zkb1tMpFNB");

#[program]
pub mod rent_example {
    use super::*;

    pub fn create_system_account(
        ctx: Context<CreateSystemAccount>,
        address_data: AddressData,
    ) -> Result<()> {
        msg!("Program invoked. Creating a system account...");
        msg!(
            "  New public key will be: {}",
            &ctx.accounts.new_account.key().to_string()
        );

        // Determine the necessary minimum rent by calculating the account's size
        //
        let account_span = (address_data.try_to_vec()?).len();
        let lamports_required = (Rent::get()?).minimum_balance(account_span);

        msg!("Account span: {}", &account_span);
        msg!("Lamports required: {}", &lamports_required);

        system_program::create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.new_account.to_account_info(),
                },
            ),
            lamports_required,
            account_span as u64,
            &ctx.accounts.system_program.key(),
        )?;

        msg!("Account created succesfully.");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateSystemAccount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub new_account: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
pub struct AddressData {
    name: String,
    address: String,
}

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/state/game_data.rs
use anchor_lang::prelude::*;

use crate::constants::MAX_WOOD_PER_TREE;

#[account]
pub struct GameData {
    pub total_wood_collected: u64,
}

impl GameData {
    pub fn on_tree_chopped(&mut self, amount_chopped: u64) -> Result<()> {
        match self.total_wood_collected.checked_add(amount_chopped) {
            Some(v) => {
                if self.total_wood_collected >= MAX_WOOD_PER_TREE {
                    self.total_wood_collected = 0;
                    msg!("Tree successfully chopped. New Tree coming up.");
                } else {
                    self.total_wood_collected = v;
                    msg!("Total wood chopped: {}", v);
                }
            }
            None => {
                msg!("The ever tree is completly chopped!");
            }
        };

        Ok(())
    }
}

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/state/mod.rs
pub mod game_data;
pub mod player_data;

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/state/player_data.rs
use crate::constants::*;
use anchor_lang::prelude::*;

#[account]
pub struct PlayerData {
    pub authority: Pubkey,
    pub name: String,
    pub level: u8,
    pub xp: u64,
    pub wood: u64,
    pub energy: u64,
    pub last_login: i64,
    pub last_id: u16,
}

impl PlayerData {
    pub fn print(&mut self) -> Result<()> {
        // Note that logging costs a lot of compute. So don't use it too much.
        msg!(
            "Authority: {} Wood: {} Energy: {}",
            self.authority,
            self.wood,
            self.energy
        );
        Ok(())
    }

    pub fn update_energy(&mut self) -> Result<()> {
        // Get the current timestamp
        let current_timestamp = Clock::get()?.unix_timestamp;

        // Calculate the time passed since the last login
        let mut time_passed: i64 = current_timestamp - self.last_login;

        // Calculate the time spent refilling energy
        let mut time_spent = 0;

        while time_passed >= TIME_TO_REFILL_ENERGY && self.energy < MAX_ENERGY {
            self.energy += 1;
            time_passed -= TIME_TO_REFILL_ENERGY;
            time_spent += TIME_TO_REFILL_ENERGY;
        }

        if self.energy >= MAX_ENERGY {
            self.last_login = current_timestamp;
        } else {
            self.last_login += time_spent;
        }

        Ok(())
    }

    pub fn chop_tree(&mut self, amount: u64) -> Result<()> {
        match self.wood.checked_add(amount) {
            Some(v) => {
                self.wood = v;
            }
            None => {
                msg!("Total wood reached!");
            }
        };
        match self.energy.checked_sub(amount) {
            Some(v) => {
                self.energy = v;
            }
            None => {
                self.energy = 0;
            }
        };
        Ok(())
    }
}

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/lib.rs
pub use crate::errors::GameErrorCode;
pub use anchor_lang::prelude::*;
pub use session_keys::{ session_auth_or, Session, SessionError };
pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
use instructions::*;

declare_id!("9aZZ7TJ2fQZxY8hMtWXywp5y6BgqC4N2BPcr9FDT47sW");

#[program]
pub mod extension_nft {
    use super::*;

    pub fn init_player(ctx: Context<InitPlayer>, _level_seed: String) -> Result<()> {
        init_player::init_player(ctx)
    }

    // This function lets the player chop a tree and get 1 wood. The session_auth_or macro
    // lets the player either use their session token or their main wallet. (The counter is only
    // there so that the player can do multiple transactions in the same block. Without it multiple transactions
    // in the same block would result in the same signature and therefore fail.)
    #[session_auth_or(
        ctx.accounts.player.authority.key() == ctx.accounts.signer.key(),
        GameErrorCode::WrongAuthority
    )]
    pub fn chop_tree(ctx: Context<ChopTree>, _level_seed: String, counter: u16) -> Result<()> {
        chop_tree::chop_tree(ctx, counter, 1)
    }

    pub fn mint_nft(ctx: Context<MintNft>) -> Result<()> {
        mint_nft::mint_nft(ctx)
    }
}

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/constants.rs
pub const TIME_TO_REFILL_ENERGY: i64 = 60;
pub const MAX_ENERGY: u64 = 100;
pub const MAX_WOOD_PER_TREE: u64 = 100000;

>>> program-examples/tokens/token-2022/group/anchor/programs/group/src/lib.rs
use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::extension::group_pointer::GroupPointer;
use anchor_spl::token_interface::{
    spl_token_2022::{
        extension::{BaseStateWithExtensions, StateWithExtensions},
        state::Mint as MintState,
    },
    token_group_initialize, Mint, Token2022, TokenGroupInitialize,
};

declare_id!("4XCDGMD8fsdjUzmYj6d9if8twFt1f23Ym52iDmWK8fFs");

#[program]
pub mod group {

    use super::*;

    pub fn test_initialize_group(ctx: Context<InitializeGroup>) -> Result<()> {
        ctx.accounts.check_mint_data()?;

        // // Token Group and Token Member extensions features not enabled yet on the Token2022 program
        // // This is temporary placeholder to update one extensions are live
        // // Initializing the "pointers" works, but you can't initialize the group/member data yet

        // let signer_seeds: &[&[&[u8]]] = &[&[b"group", &[ctx.bumps.mint_account]]];
        // token_group_initialize(
        //     CpiContext::new(
        //         ctx.accounts.token_program.to_account_info(),
        //         TokenGroupInitialize {
        //             token_program_id: ctx.accounts.token_program.to_account_info(),
        //             group: ctx.accounts.mint_account.to_account_info(),
        //             mint: ctx.accounts.mint_account.to_account_info(),
        //             mint_authority: ctx.accounts.mint_account.to_account_info(),
        //         },
        //     )
        //     .with_signer(signer_seeds),
        //     Some(ctx.accounts.payer.key()), // update_authority
        //     10,                             // max_size
        // )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeGroup<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        seeds = [b"group"],
        bump,
        payer = payer,
        mint::decimals = 2,
        mint::authority = mint_account,
        mint::freeze_authority = mint_account,
        extensions::group_pointer::authority = mint_account,
        extensions::group_pointer::group_address = mint_account,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeGroup<'info> {
    pub fn check_mint_data(&self) -> Result<()> {
        let mint = &self.mint_account.to_account_info();
        let mint_data = mint.data.borrow();
        let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
        let extension_data = mint_with_extension.get_extension::<GroupPointer>()?;

        msg!("{:?}", mint_with_extension);
        msg!("{:?}", extension_data);
        Ok(())
    }
}

>>> program-examples/basics/program-derived-addresses/anchor/programs/anchor-program-example/src/instructions/mod.rs
pub mod create;
pub mod increment;

pub use create::*;
pub use increment::*;

>>> program-examples/basics/program-derived-addresses/anchor/programs/anchor-program-example/src/instructions/create.rs
use crate::state::PageVisits;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreatePageVisits<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(
        init,
        space = 8 + PageVisits::INIT_SPACE,
        payer = payer,
        seeds = [
            PageVisits::SEED_PREFIX,
            payer.key().as_ref(),
        ],
        bump,
    )]
    page_visits: Account<'info, PageVisits>,
    system_program: Program<'info, System>,
}

pub fn create_page_visits(ctx: Context<CreatePageVisits>) -> Result<()> {
    *ctx.accounts.page_visits = PageVisits {
        page_visits: 0,
        bump: ctx.bumps.page_visits,
    };

    Ok(())
}

>>> program-examples/basics/program-derived-addresses/anchor/programs/anchor-program-example/src/instructions/increment.rs
use crate::state::PageVisits;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct IncrementPageVisits<'info> {
    user: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [
            PageVisits::SEED_PREFIX,
            user.key().as_ref(),
        ],
        bump = page_visits.bump,
    )]
    page_visits: Account<'info, PageVisits>,
}

pub fn increment_page_visits(ctx: Context<IncrementPageVisits>) -> Result<()> {
    let page_visits = &mut ctx.accounts.page_visits;
    page_visits.increment();
    Ok(())
}

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/instructions/init_player.rs
pub use crate::errors::GameErrorCode;
use crate::state::player_data::PlayerData;
use crate::{constants::MAX_ENERGY, GameData};
use anchor_lang::prelude::*;

pub fn init_player(ctx: Context<InitPlayer>) -> Result<()> {
    ctx.accounts.player.energy = MAX_ENERGY;
    ctx.accounts.player.last_login = Clock::get()?.unix_timestamp;
    ctx.accounts.player.authority = ctx.accounts.signer.key();
    Ok(())
}

#[derive(Accounts)]
#[instruction(level_seed: String)]
pub struct InitPlayer<'info> {
    #[account(
        init,
        payer = signer,
        space = 1000, // 8+32+x+1+8+8+8 But taking 1000 to have space to expand easily.
        seeds = [b"player".as_ref(), signer.key().as_ref()],
        bump,
    )]
    pub player: Account<'info, PlayerData>,

    #[account(
        init_if_needed,
        payer = signer,
        space = 1000, // 8 + 8 for anchor account discriminator and the u64. Using 1000 to have space to expand easily.
        seeds = [level_seed.as_ref()],
        bump,
    )]
    pub game_data: Account<'info, GameData>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/instructions/chop_tree.rs
pub use crate::errors::GameErrorCode;
pub use crate::state::game_data::GameData;
use crate::{state::player_data::PlayerData, NftAuthority};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Token2022};
use session_keys::{Session, SessionToken};
use solana_program::program::invoke_signed;

pub fn chop_tree(ctx: Context<ChopTree>, counter: u16, amount: u64) -> Result<()> {
    let account: &mut ChopTree<'_> = ctx.accounts;
    account.player.update_energy()?;
    account.player.print()?;

    if account.player.energy < amount {
        return err!(GameErrorCode::NotEnoughEnergy);
    }

    account.player.last_id = counter;
    account.player.chop_tree(amount)?;
    account.game_data.on_tree_chopped(amount)?;

    msg!(
        "You chopped a tree and got 1 wood. You have {} wood and {} energy left.",
        ctx.accounts.player.wood,
        ctx.accounts.player.energy
    );

    // We use a PDA as a mint authority for the metadata account because we want to be able to update the NFT from
    // the program.
    let seeds = b"nft_authority";
    let bump = ctx.bumps.nft_authority;
    let signer: &[&[&[u8]]] = &[&[seeds, &[bump]]];

    // Update the metadata account with an additional metadata field in this case the player level
    invoke_signed(
        &spl_token_metadata_interface::instruction::update_field(
            &spl_token_2022::id(),
            ctx.accounts.mint.to_account_info().key,
            ctx.accounts.nft_authority.to_account_info().key,
            spl_token_metadata_interface::state::Field::Key("wood".to_string()),
            ctx.accounts.player.wood.to_string(),
        ),
        &[
            ctx.accounts.mint.to_account_info().clone(),
            ctx.accounts.nft_authority.to_account_info().clone(),
        ],
        signer,
    )?;

    Ok(())
}

#[derive(Accounts, Session)]
#[instruction(level_seed: String)]
pub struct ChopTree<'info> {
    #[session(
        // The ephemeral key pair signing the transaction
        signer = signer,
        // The authority of the user account which must have created the session
        authority = player.authority.key()
    )]
    // Session Tokens are passed as optional accounts
    pub session_token: Option<Account<'info, SessionToken>>,

    // There is one PlayerData account
    #[account(
        mut,
        seeds = [b"player".as_ref(), player.authority.key().as_ref()],
        bump,
    )]
    pub player: Account<'info, PlayerData>,

    // There can be multiple levels the seed for the level is passed in the instruction
    // First player starting a new level will pay for the account in the current setup
    #[account(
        init_if_needed,
        payer = signer,
        space = 1000,
        seeds = [level_seed.as_ref()],
        bump,
    )]
    pub game_data: Account<'info, GameData>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Make sure the ata to the mint is actually owned by the signer
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(  
        init_if_needed,
        seeds = [b"nft_authority".as_ref()],
        bump,
        space = 8,
        payer = signer,
    )]
    pub nft_authority: Account<'info, NftAuthority>,
    pub token_program: Program<'info, Token2022>,
}

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/instructions/mod.rs
//! All instructions
pub mod chop_tree;
pub mod init_player;
pub mod mint_nft;

pub use chop_tree::*;
pub use init_player::*;
pub use mint_nft::*;

>>> program-examples/tokens/token-2022/nft-meta-data-pointer/anchor-example/anchor/programs/extension_nft/src/instructions/mint_nft.rs
pub use crate::errors::GameErrorCode;
pub use crate::errors::ProgramErrorCode;
pub use crate::state::game_data::GameData;
use anchor_lang::{ prelude::*, system_program };
use anchor_spl::{
    associated_token::{ self, AssociatedToken },
    token_2022,
    token_interface::{ spl_token_2022::instruction::AuthorityType, Token2022 },
};
use solana_program::program::{ invoke, invoke_signed };
use spl_token_2022::{ extension::ExtensionType, state::Mint };

pub fn mint_nft(ctx: Context<MintNft>) -> Result<()> {
    msg!("Mint nft with meta data extension and additional meta data");

    let space = match
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
    {
        Ok(space) => space,
        Err(_) => {
            return err!(ProgramErrorCode::InvalidMintAccountSpace);
        }
    };

    // This is the space required for the metadata account.
    // We put the meta data into the mint account at the end so we
    // don't need to create and additional account.
    let meta_data_space = 250;

    let lamports_required = Rent::get()?.minimum_balance(space + meta_data_space);

    msg!(
        "Create Mint and metadata account size and cost: {} lamports: {}",
        space as u64,
        lamports_required
    );

    system_program::create_account(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            system_program::CreateAccount {
                from: ctx.accounts.signer.to_account_info(),
                to: ctx.accounts.mint.to_account_info(),
            }
        ),
        lamports_required,
        space as u64,
        &ctx.accounts.token_program.key()
    )?;

    // Assign the mint to the token program
    system_program::assign(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), system_program::Assign {
            account_to_assign: ctx.accounts.mint.to_account_info(),
        }),
        &token_2022::ID
    )?;

    // Initialize the metadata pointer (Need to do this before initializing the mint)
    let init_meta_data_pointer_ix = match
        spl_token_2022::extension::metadata_pointer::instruction::initialize(
            &Token2022::id(),
            &ctx.accounts.mint.key(),
            Some(ctx.accounts.nft_authority.key()),
            Some(ctx.accounts.mint.key())
        )
    {
        Ok(ix) => ix,
        Err(_) => {
            return err!(ProgramErrorCode::CantInitializeMetadataPointer);
        }
    };

    invoke(
        &init_meta_data_pointer_ix,
        &[ctx.accounts.mint.to_account_info(), ctx.accounts.nft_authority.to_account_info()]
    )?;

    // Initialize the mint cpi
    let mint_cpi_ix = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token_2022::InitializeMint2 {
            mint: ctx.accounts.mint.to_account_info(),
        }
    );

    token_2022::initialize_mint2(mint_cpi_ix, 0, &ctx.accounts.nft_authority.key(), None).unwrap();

    // We use a PDA as a mint authority for the metadata account because
    // we want to be able to update the NFT from the program.
    let seeds = b"nft_authority";
    let bump = ctx.bumps.nft_authority;
    let signer: &[&[&[u8]]] = &[&[seeds, &[bump]]];

    msg!("Init metadata {0}", ctx.accounts.nft_authority.to_account_info().key);

    // Init the metadata account
    let init_token_meta_data_ix = &spl_token_metadata_interface::instruction::initialize(
        &spl_token_2022::id(),
        ctx.accounts.mint.key,
        ctx.accounts.nft_authority.to_account_info().key,
        ctx.accounts.mint.key,
        ctx.accounts.nft_authority.to_account_info().key,
        "Beaver".to_string(),
        "BVA".to_string(),
        "https://arweave.net/MHK3Iopy0GgvDoM7LkkiAdg7pQqExuuWvedApCnzfj0".to_string()
    );

    invoke_signed(
        init_token_meta_data_ix,
        &[
            ctx.accounts.mint.to_account_info().clone(),
            ctx.accounts.nft_authority.to_account_info().clone(),
        ],
        signer
    )?;

    // Update the metadata account with an additional metadata field in this case the player level
    invoke_signed(
        &spl_token_metadata_interface::instruction::update_field(
            &spl_token_2022::id(),
            ctx.accounts.mint.key,
            ctx.accounts.nft_authority.to_account_info().key,
            spl_token_metadata_interface::state::Field::Key("level".to_string()),
            "1".to_string()
        ),
        &[
            ctx.accounts.mint.to_account_info().clone(),
            ctx.accounts.nft_authority.to_account_info().clone(),
        ],
        signer
    )?;

    // Create the associated token account
    associated_token::create(
        CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            associated_token::Create {
                payer: ctx.accounts.signer.to_account_info(),
                associated_token: ctx.accounts.token_account.to_account_info(),
                authority: ctx.accounts.signer.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            }
        )
    )?;

    // Mint one token to the associated token account of the player
    token_2022::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token_2022::MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.token_account.to_account_info(),
                authority: ctx.accounts.nft_authority.to_account_info(),
            },
            signer
        ),
        1
    )?;

    // Freeze the mint authority so no more tokens can be minted to make it an NFT
    token_2022::set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token_2022::SetAuthority {
                current_authority: ctx.accounts.nft_authority.to_account_info(),
                account_or_mint: ctx.accounts.mint.to_account_info(),
            },
            signer
        ),
        AuthorityType::MintTokens,
        None
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct MintNft<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>,
    /// CHECK: We will create this one for the user
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    #[account(mut)]
    pub mint: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(init_if_needed, seeds = [b"nft_authority".as_ref()], bump, space = 8, payer = signer)]
    pub nft_authority: Account<'info, NftAuthority>,
}

#[account]
pub struct NftAuthority {}

>>> program-examples/tokens/token-2022/transfer-fee/anchor/programs/transfer-fee/src/lib.rs
use anchor_lang::prelude::*;

mod instructions;
use instructions::*;

declare_id!("4evptdGtALCNT8uTxJhbWBRZpBE8w5oNtmgfSyfQu7td");

#[program]
pub mod transfer_fee {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        process_initialize(ctx, transfer_fee_basis_points, maximum_fee)
    }

    pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        process_transfer(ctx, amount)
    }

    pub fn harvest<'info>(ctx: Context<'_, '_, 'info, 'info, Harvest<'info>>) -> Result<()> {
        process_harvest(ctx)
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        process_withdraw(ctx)
    }

    pub fn update_fee(
        ctx: Context<UpdateFee>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        process_update_fee(ctx, transfer_fee_basis_points, maximum_fee)
    }
}

>>> program-examples/basics/checking-accounts/anchor/programs/anchor-program-example/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

declare_id!("ECWPhR3rJbaPfyNFgphnjxSEexbTArc7vxD8fnW6tgKw");

#[program]
pub mod anchor_program_example {
    use super::*;

    pub fn check_accounts(_ctx: Context<CheckingAccounts>) -> Result<()> {
        Ok(())
    }
}

// Account validation in Anchor is done using the types and constraints specified in the #[derive(Accounts)] structs
// This is a simple example and does not include all possible constraints and types
#[derive(Accounts)]
pub struct CheckingAccounts<'info> {
    payer: Signer<'info>, // checks account is signer

    /// CHECK: No checks performed, example of an unchecked account
    #[account(mut)]
    account_to_create: UncheckedAccount<'info>,
    /// CHECK: Perform owner check using constraint
    #[account(
        mut,
        owner = id()
    )]
    account_to_change: UncheckedAccount<'info>,
    system_program: Program<'info, System>, // checks account is executable, and is the system program
}

>>> program-examples/tokens/token-2022/transfer-fee/anchor/programs/transfer-fee/src/instructions/transfer.rs
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::spl_token_2022::{
        extension::{
            transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions,
        },
        state::Mint as MintState,
    },
    token_interface::{
        transfer_checked_with_fee, Mint, Token2022, TokenAccount, TransferCheckedWithFee,
    },
};

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    pub recipient: SystemAccount<'info>,

    #[account(mut)]
    pub mint_account: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = sender,
        associated_token::token_program = token_program
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = sender,
        associated_token::mint = mint_account,
        associated_token::authority = recipient,
        associated_token::token_program = token_program
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// transfer fees are automatically deducted from the transfer amount
// recipients receives (transfer amount - fees)
// transfer fees are stored directly on the recipient token account and must be "harvested"
pub fn process_transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
    // read mint account extension data
    let mint = &ctx.accounts.mint_account.to_account_info();
    let mint_data = mint.data.borrow();
    let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
    let extension_data = mint_with_extension.get_extension::<TransferFeeConfig>()?;

    // calculate expected fee
    let epoch = Clock::get()?.epoch;
    let fee = extension_data.calculate_epoch_fee(epoch, amount).unwrap();

    // mint account decimals
    let decimals = ctx.accounts.mint_account.decimals;

    transfer_checked_with_fee(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferCheckedWithFee {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                source: ctx.accounts.sender_token_account.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                destination: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.sender.to_account_info(),
            },
        ),
        amount,   // transfer amount
        decimals, // decimals
        fee,      // fee
    )?;

    msg!("transfer amount {}", amount);
    msg!("fee amount {}", fee);

    Ok(())
}

>>> program-examples/tokens/token-2022/transfer-fee/anchor/programs/transfer-fee/src/instructions/harvest.rs
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    harvest_withheld_tokens_to_mint, HarvestWithheldTokensToMint, Mint, Token2022, TokenAccount,
};

#[derive(Accounts)]
pub struct Harvest<'info> {
    #[account(mut)]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
}

// transfer fees are stored directly on the recipient token account and must be "harvested"
// "harvesting" transfers fees accumulated on token accounts to the mint account
pub fn process_harvest<'info>(ctx: Context<'_, '_, 'info, 'info, Harvest<'info>>) -> Result<()> {
    // Using remaining accounts to allow for passing in an unknown number of token accounts to harvest from
    // Check that remaining accounts are token accounts for the mint to harvest to
    let sources = ctx
        .remaining_accounts
        .iter()
        .filter_map(|account| {
            InterfaceAccount::<TokenAccount>::try_from(account)
                .ok()
                .filter(|token_account| token_account.mint == ctx.accounts.mint_account.key())
                .map(|_| account.to_account_info())
        })
        .collect::<Vec<_>>();

    harvest_withheld_tokens_to_mint(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            HarvestWithheldTokensToMint {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
            },
        ),
        sources, // token accounts to harvest from
    )?;
    Ok(())
}

>>> program-examples/tokens/token-2022/transfer-fee/anchor/programs/transfer-fee/src/instructions/update_fee.rs
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{transfer_fee_set, Mint, Token2022, TransferFeeSetTransferFee};

#[derive(Accounts)]
pub struct UpdateFee<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
}

// Note that there is a 2 epoch delay from when new fee updates take effect
// This is a safely feature built into the extension
// https://github.com/solana-labs/solana-program-library/blob/master/token/program-2022/src/extension/transfer_fee/processor.rs#L92-L109
pub fn process_update_fee(
    ctx: Context<UpdateFee>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    transfer_fee_set(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferFeeSetTransferFee {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        transfer_fee_basis_points, // transfer fee basis points (% fee per transfer)
        maximum_fee,               // maximum fee (maximum units of token per transfer)
    )?;
    Ok(())
}

>>> program-examples/tokens/token-2022/transfer-fee/anchor/programs/transfer-fee/src/instructions/mod.rs
pub mod transfer;
pub use transfer::*;
pub mod initialize;
pub use initialize::*;
pub mod harvest;
pub use harvest::*;
pub mod withdraw;
pub use withdraw::*;
pub mod update_fee;
pub use update_fee::*;

>>> program-examples/tokens/token-2022/transfer-fee/anchor/programs/transfer-fee/src/instructions/initialize.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    token_2022::{
        initialize_mint2,
        spl_token_2022::{
            extension::{
                transfer_fee::TransferFeeConfig, BaseStateWithExtensions, ExtensionType,
                StateWithExtensions,
            },
            pod::PodMint,
            state::Mint as MintState,
        },
        InitializeMint2,
    },
    token_interface::{
        spl_pod::optional_keys::OptionalNonZeroPubkey, transfer_fee_initialize, Token2022,
        TransferFeeInitialize,
    },
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub mint_account: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

// There is currently not an anchor constraint to automatically initialize the TransferFeeConfig extension
// We can manually create and initialize the mint account via CPIs in the instruction handler
pub fn process_initialize(
    ctx: Context<Initialize>,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    // Calculate space required for mint and extension data
    let mint_size =
        ExtensionType::try_calculate_account_len::<PodMint>(&[ExtensionType::TransferFeeConfig])?;

    // Calculate minimum lamports required for size of mint account with extensions
    let lamports = (Rent::get()?).minimum_balance(mint_size);

    // Invoke System Program to create new account with space for mint and extension data
    create_account(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            CreateAccount {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.mint_account.to_account_info(),
            },
        ),
        lamports,                          // Lamports
        mint_size as u64,                  // Space
        &ctx.accounts.token_program.key(), // Owner Program
    )?;

    // Initialize the transfer fee extension data
    // This instruction must come before the instruction to initialize the mint data
    transfer_fee_initialize(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferFeeInitialize {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
            },
        ),
        Some(&ctx.accounts.payer.key()), // transfer fee config authority (update fee)
        Some(&ctx.accounts.payer.key()), // withdraw authority (withdraw fees)
        transfer_fee_basis_points,       // transfer fee basis points (% fee per transfer)
        maximum_fee,                     // maximum fee (maximum units of token per transfer)
    )?;

    // Initialize the standard mint account data
    initialize_mint2(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            InitializeMint2 {
                mint: ctx.accounts.mint_account.to_account_info(),
            },
        ),
        2,                               // decimals
        &ctx.accounts.payer.key(),       // mint authority
        Some(&ctx.accounts.payer.key()), // freeze authority
    )?;

    ctx.accounts.check_mint_data()?;
    Ok(())
}

// helper to demonstrate how to read mint extension data within a program
impl<'info> Initialize<'info> {
    pub fn check_mint_data(&self) -> Result<()> {
        let mint = &self.mint_account.to_account_info();
        let mint_data = mint.data.borrow();
        let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
        let extension_data = mint_with_extension.get_extension::<TransferFeeConfig>()?;

        assert_eq!(
            extension_data.transfer_fee_config_authority,
            OptionalNonZeroPubkey::try_from(Some(self.payer.key()))?
        );

        assert_eq!(
            extension_data.withdraw_withheld_authority,
            OptionalNonZeroPubkey::try_from(Some(self.payer.key()))?
        );

        msg!("{:?}", extension_data);
        Ok(())
    }
}

>>> program-examples/tokens/token-2022/transfer-fee/anchor/programs/transfer-fee/src/instructions/withdraw.rs
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    withdraw_withheld_tokens_from_mint, Mint, Token2022, TokenAccount,
    WithdrawWithheldTokensFromMint,
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub mint_account: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token2022>,
}

// transfer fees "harvested" to the mint account can then be withdraw by the withdraw authority
// this transfers fees on the mint account to the specified token account
pub fn process_withdraw(ctx: Context<Withdraw>) -> Result<()> {
    withdraw_withheld_tokens_from_mint(CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        WithdrawWithheldTokensFromMint {
            token_program_id: ctx.accounts.token_program.to_account_info(),
            mint: ctx.accounts.mint_account.to_account_info(),
            destination: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        },
    ))?;
    Ok(())
}

>>> program-examples/basics/account-data/anchor/programs/anchor-program-example/src/state/address_info.rs
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)] // automatically calculate the space required for the struct
pub struct AddressInfo {
    #[max_len(50)] // set a max length for the string
    pub name: String, // 4 bytes + 50 bytes
    pub house_number: u8, // 1 byte
    #[max_len(50)]
    pub street: String, // 4 bytes + 50 bytes
    #[max_len(50)]
    pub city: String, // 4 bytes + 50 bytes
}

>>> program-examples/basics/account-data/anchor/programs/anchor-program-example/src/state/mod.rs
pub mod address_info;

pub use address_info::*;

>>> program-examples/basics/account-data/anchor/programs/anchor-program-example/src/lib.rs
#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use instructions::*;

pub mod constants;
pub mod instructions;
pub mod state;

declare_id!("GpVcgWdgVErgLqsn8VYUch6EqDerMgNqoLSmGyKrd6MR");

#[program]
pub mod anchor_program_example {
    use super::*;

    pub fn create_address_info(
        ctx: Context<CreateAddressInfo>,
        name: String,
        house_number: u8,
        street: String,
        city: String,
    ) -> Result<()> {
        create::create_address_info(ctx, name, house_number, street, city)
    }
}

>>> program-examples/basics/account-data/anchor/programs/anchor-program-example/src/constants.rs
pub const ANCHOR_DISCRIMINATOR_SIZE: usize = 8;

>>> program-examples/tokens/token-2022/transfer-hook/counter/anchor/programs/transfer-hook/src/lib.rs
use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{
        spl_token_2022::{
            extension::{
                transfer_hook::TransferHookAccount,
                BaseStateWithExtensionsMut,
                PodStateWithExtensionsMut,
            },
            pod::PodAccount,
        },
        Token2022,
    },
    token_interface::{ Mint, TokenAccount },
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    seeds::Seed,
    state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

declare_id!("1qahDxKHeCLZhbBU2NyMU6vQCQmEUmdeSEBrG5drffK");

#[error_code]
pub enum TransferError {
    #[msg("The amount is too big")]
    AmountTooBig,
    #[msg("The token is not currently transferring")]
    IsNotCurrentlyTransferring,
}

#[program]
pub mod transfer_hook {
    use super::*;

    #[interface(spl_transfer_hook_interface::initialize_extra_account_meta_list)]
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>
    ) -> Result<()> {
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas
        )?;

        Ok(())
    }

    #[interface(spl_transfer_hook_interface::execute)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook
        check_is_transferring(&ctx)?;

        // Check if the amount is too big
        if amount > 50 {
            msg!("The amount is too big: {}", amount);
            //return err!(TransferError::AmountTooBig);
        }

        // Increment the transfer count safely
        let count = ctx.accounts.counter_account.counter
            .checked_add(1)
            .ok_or(TransferError::AmountTooBig)?;

        msg!("This token has been transferred {} times", count);

        Ok(())
    }
}

fn check_is_transferring(ctx: &Context<TransferHook>) -> Result<()> {
    let source_token_info = ctx.accounts.source_token.to_account_info();
    let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
    let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
    let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

    if !bool::from(account_extension.transferring) {
        return err!(TransferError::IsNotCurrentlyTransferring);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        )?,
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(init, seeds = [b"counter"], bump, payer = payer, space = 16)]
    pub counter_account: Account<'info, CounterAccount>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// Define extra account metas to store on extra_account_meta_list account
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        Ok(
            vec![
                ExtraAccountMeta::new_with_seeds(
                    &[
                        Seed::Literal {
                            bytes: b"counter".to_vec(),
                        },
                    ],
                    false, // is_signer
                    true // is_writable
                )?
            ]
        )
    }
}

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint, token::authority = owner)]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(token::mint = mint)]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(seeds = [b"counter"], bump)]
    pub counter_account: Account<'info, CounterAccount>,
}

#[account]
pub struct CounterAccount {
    counter: u64,
}

>>> program-examples/basics/account-data/anchor/programs/anchor-program-example/src/instructions/mod.rs
pub mod create;
pub use create::*;

>>> program-examples/basics/account-data/anchor/programs/anchor-program-example/src/instructions/create.rs
use crate::{constants::ANCHOR_DISCRIMINATOR_SIZE, state::AddressInfo};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreateAddressInfo<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = ANCHOR_DISCRIMINATOR_SIZE + AddressInfo::INIT_SPACE,
    )]
    address_info: Account<'info, AddressInfo>,
    system_program: Program<'info, System>,
}

pub fn create_address_info(
    ctx: Context<CreateAddressInfo>,
    name: String,
    house_number: u8,
    street: String,
    city: String,
) -> Result<()> {
    *ctx.accounts.address_info = AddressInfo {
        name,
        house_number,
        street,
        city,
    };
    Ok(())
}

>>> program-examples/basics/counter/anchor/programs/counter_anchor/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

declare_id!("BmDHboaj1kBUoinJKKSRqKfMeRKJqQqEbUj1VgzeQe4A");

#[program]
pub mod counter_anchor {
    use super::*;

    pub fn initialize_counter(_ctx: Context<InitializeCounter>) -> Result<()> {
        Ok(())
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        ctx.accounts.counter.count = ctx.accounts.counter.count.checked_add(1).unwrap();
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeCounter<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        space = 8 + Counter::INIT_SPACE,
        payer = payer
    )]
    pub counter: Account<'info, Counter>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut)]
    pub counter: Account<'info, Counter>,
}

#[account]
#[derive(InitSpace)]
pub struct Counter {
    count: u64,
}

>>> program-examples/basics/close-account/anchor/programs/close-account/src/state/mod.rs
pub mod user_state;
pub use user_state::*;

>>> program-examples/basics/close-account/anchor/programs/close-account/src/state/user_state.rs
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)] // automatically calculate the space required for the struct
pub struct UserState {
    pub bump: u8,     // 1 byte
    pub user: Pubkey, // 32 bytes
    #[max_len(50)] // set a max length for the string
    pub name: String, // 4 bytes + 50 bytes
}

>>> program-examples/basics/close-account/anchor/programs/close-account/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
mod instructions;
mod state;
use instructions::*;

declare_id!("99TQtoDdQ5NS2v5Ppha93aqEmv3vV9VZVfHTP5rGST3c");

#[program]
pub mod close_account_program {
    use super::*;

    pub fn create_user(ctx: Context<CreateUserContext>, name: String) -> Result<()> {
        create_user::create_user(ctx, name)
    }

    pub fn close_user(ctx: Context<CloseUserContext>) -> Result<()> {
        close_user::close_user(ctx)
    }
}

>>> program-examples/basics/realloc/anchor/programs/anchor-realloc/src/lib.rs
use anchor_lang::prelude::*;

declare_id!("Fod47xKXjdHVQDzkFPBvfdWLm8gEAV4iMSXkfUzCHiSD");

#[program]
pub mod anchor_realloc {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, input: String) -> Result<()> {
        ctx.accounts.message_account.message = input;
        Ok(())
    }

    pub fn update(ctx: Context<Update>, input: String) -> Result<()> {
        ctx.accounts.message_account.message = input;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(input: String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = Message::required_space(input.len()),
    )]
    pub message_account: Account<'info, Message>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(input: String)]
pub struct Update<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        realloc = Message::required_space(input.len()),
        realloc::payer = payer,
        realloc::zero = true,
    )]
    pub message_account: Account<'info, Message>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Message {
    pub message: String,
}

impl Message {
    pub fn required_space(input_len: usize) -> usize {
        8 + // 8 byte discriminator
        4 + // 4 byte for length of string
        input_len
    }
}

>>> program-examples/basics/close-account/anchor/programs/close-account/src/instructions/close_user.rs
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CloseUserContext<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"USER",
            user.key().as_ref(),
        ],
        bump = user_account.bump,
        close = user, // close account and return lamports to user
    )]
    pub user_account: Account<'info, UserState>,
}

pub fn close_user(_ctx: Context<CloseUserContext>) -> Result<()> {
    Ok(())
}

>>> program-examples/basics/close-account/anchor/programs/close-account/src/instructions/mod.rs
pub mod close_user;
pub mod create_user;

pub use close_user::*;
pub use create_user::*;

>>> program-examples/basics/close-account/anchor/programs/close-account/src/instructions/create_user.rs
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreateUserContext<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = UserState::INIT_SPACE,
        seeds = [
            b"USER",
            user.key().as_ref(),
        ],
        bump
    )]
    pub user_account: Account<'info, UserState>,
    pub system_program: Program<'info, System>,
}

pub fn create_user(ctx: Context<CreateUserContext>, name: String) -> Result<()> {
    *ctx.accounts.user_account = UserState {
        bump: ctx.bumps.user_account,
        user: ctx.accounts.user.key(),
        name,
    };
    Ok(())
}

>>> program-examples/tokens/token-2022/transfer-hook/account-data-as-seed/anchor/programs/transfer-hook/src/lib.rs
use std::cell::RefMut;

use anchor_lang::{ prelude::* };
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{
        spl_token_2022::{
            extension::{
                transfer_hook::TransferHookAccount,
                BaseStateWithExtensionsMut,
                PodStateWithExtensionsMut,
            },
            pod::PodAccount,
        },
        Token2022,
    },
    token_interface::{ Mint, TokenAccount },
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    seeds::Seed,
    state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

declare_id!("1qahDxKHeCLZhbBU2NyMU6vQCQmEUmdeSEBrG5drffK");

#[error_code]
pub enum TransferError {
    #[msg("The amount is too big")]
    AmountTooBig,
    #[msg("The token is not currently transferring")]
    IsNotCurrentlyTransferring,
}

#[program]
pub mod transfer_hook {
    use super::*;

    #[interface(spl_transfer_hook_interface::initialize_extra_account_meta_list)]
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>
    ) -> Result<()> {
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas
        )?;

        Ok(())
    }

    #[interface(spl_transfer_hook_interface::execute)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook
        check_is_transferring(&ctx)?;

        // Check if the amount is too big
        if amount > 50 {
            msg!("The amount is too big: {}", amount);
            //return err!(TransferError::AmountTooBig);
        }

        // Increment the transfer count safely
        let count = ctx.accounts.counter_account.counter
            .checked_add(1)
            .ok_or(TransferError::AmountTooBig)?;

        msg!("This token has been transferred {} times", count);

        Ok(())
    }
}

fn check_is_transferring(ctx: &Context<TransferHook>) -> Result<()> {
    let source_token_info = ctx.accounts.source_token.to_account_info();
    let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
    let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
    let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

    if !bool::from(account_extension.transferring) {
        return err!(TransferError::IsNotCurrentlyTransferring);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        )?,
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(init, seeds = [b"counter", payer.key().as_ref()], bump, payer = payer, space = 16)]
    pub counter_account: Account<'info, CounterAccount>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// Define extra account metas to store on extra_account_meta_list account
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        Ok(
            vec![
                ExtraAccountMeta::new_with_seeds(
                    &[
                        Seed::Literal {
                            bytes: b"counter".to_vec(),
                        },
                        Seed::AccountData { account_index: 0, data_index: 32, length: 32 },
                    ],
                    false, // is_signer
                    true // is_writable
                )?
            ]
        )
    }
}

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint, token::authority = owner)]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(token::mint = mint)]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(seeds = [b"counter", owner.key().as_ref()], bump)]
    pub counter_account: Account<'info, CounterAccount>,
}

#[account]
pub struct CounterAccount {
    counter: u64,
}

>>> program-examples/basics/transfer-sol/anchor/programs/transfer-sol/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("4fQVnLWKKKYxtxgGn7Haw8v2g2Hzbu8K61JvWKvqAi7W");

#[program]
pub mod transfer_sol {
    use super::*;

    pub fn transfer_sol_with_cpi(ctx: Context<TransferSolWithCpi>, amount: u64) -> Result<()> {
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.recipient.to_account_info(),
                },
            ),
            amount,
        )?;

        Ok(())
    }

    // Directly modifying lamports is only possible if the program is the owner of the account
    pub fn transfer_sol_with_program(
        ctx: Context<TransferSolWithProgram>,
        amount: u64,
    ) -> Result<()> {
        **ctx.accounts.payer.try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.recipient.try_borrow_mut_lamports()? += amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct TransferSolWithCpi<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferSolWithProgram<'info> {
    /// CHECK: Use owner constraint to check account is owned by our program
    #[account(
        mut,
        owner = id() // value of declare_id!()
    )]
    payer: UncheckedAccount<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
}

>>> program-examples/basics/create-account/anchor/programs/create-system-account/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};

declare_id!("ARVNCsYKDQsCLHbwUTJLpFXVrJdjhWZStyzvxmKe2xHi");

#[program]
pub mod create_system_account {
    use super::*;

    pub fn create_system_account(ctx: Context<CreateSystemAccount>) -> Result<()> {
        msg!("Program invoked. Creating a system account...");
        msg!(
            "  New public key will be: {}",
            &ctx.accounts.new_account.key().to_string()
        );

        // The minimum lamports for rent exemption
        let lamports = (Rent::get()?).minimum_balance(0);

        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(), // From pubkey
                    to: ctx.accounts.new_account.to_account_info(), // To pubkey
                },
            ),
            lamports,                           // Lamports
            0,                                  // Space
            &ctx.accounts.system_program.key(), // Owner Program
        )?;

        msg!("Account created succesfully.");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateSystemAccount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub new_account: Signer<'info>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-2022/default-account-state/anchor/programs/default-account-state/src/lib.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    token_2022::{
        initialize_mint2,
        spl_token_2022::{extension::ExtensionType, pod::PodMint, state::AccountState},
        InitializeMint2,
    },
    token_interface::{
        default_account_state_initialize, default_account_state_update,
        DefaultAccountStateInitialize, DefaultAccountStateUpdate, Mint, Token2022,
    },
};

declare_id!("5LdYbHiUsFxVG8bfqoeBkhBYMRmWZb3BoLuABgYW7coB");

#[program]
pub mod default_account_state {
    use super::*;

    // There is currently not an anchor constraint to automatically initialize the DefaultAccountState extension
    // We can manually create and initialize the mint account via CPIs in the instruction handler
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        // Calculate space required for mint and extension data
        let mint_size = ExtensionType::try_calculate_account_len::<PodMint>(&[
            ExtensionType::DefaultAccountState,
        ])?;

        // Calculate minimum lamports required for size of mint account with extensions
        let lamports = (Rent::get()?).minimum_balance(mint_size);

        // Invoke System Program to create new account with space for mint and extension data
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            lamports,                          // Lamports
            mint_size as u64,                  // Space
            &ctx.accounts.token_program.key(), // Owner Program
        )?;

        // Initialize the NonTransferable extension
        // This instruction must come before the instruction to initialize the mint data
        default_account_state_initialize(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                DefaultAccountStateInitialize {
                    token_program_id: ctx.accounts.token_program.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            &AccountState::Frozen, // default frozen token accounts
        )?;

        // Initialize the standard mint account data
        initialize_mint2(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                InitializeMint2 {
                    mint: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            2,                               // decimals
            &ctx.accounts.payer.key(),       // mint authority
            Some(&ctx.accounts.payer.key()), // freeze authority
        )?;
        Ok(())
    }

    pub fn update_default_state(
        ctx: Context<UpdateDefaultState>,
        account_state: AnchorAccountState,
    ) -> Result<()> {
        // Convert AnchorAccountState to spl_token_2022::state::AccountState
        let account_state = account_state.to_spl_account_state();

        default_account_state_update(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                DefaultAccountStateUpdate {
                    token_program_id: ctx.accounts.token_program.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    freeze_authority: ctx.accounts.freeze_authority.to_account_info(),
                },
            ),
            &account_state,
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub mint_account: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateDefaultState<'info> {
    #[account(mut)]
    pub freeze_authority: Signer<'info>,
    #[account(
        mut,
        mint::freeze_authority = freeze_authority,
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

// Custom enum to implement AnchorSerialize and AnchorDeserialize
// This is required to pass the enum as an argument to the instruction
#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum AnchorAccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

// Implement conversion from AnchorAccountState to spl_token_2022::state::AccountState
impl AnchorAccountState {
    pub fn to_spl_account_state(&self) -> AccountState {
        match self {
            AnchorAccountState::Uninitialized => AccountState::Uninitialized,
            AnchorAccountState::Initialized => AccountState::Initialized,
            AnchorAccountState::Frozen => AccountState::Frozen,
        }
    }
}

>>> program-examples/basics/cross-program-invocation/anchor/programs/lever/src/lib.rs
use anchor_lang::prelude::*;

declare_id!("E64FVeubGC4NPNF2UBJYX4AkrVowf74fRJD9q6YhwstN");

#[program]
pub mod lever {
    use super::*;

    pub fn initialize(_ctx: Context<InitializeLever>) -> Result<()> {
        Ok(())
    }

    pub fn switch_power(ctx: Context<SetPowerStatus>, name: String) -> Result<()> {
        let power = &mut ctx.accounts.power;
        power.is_on = !power.is_on;

        msg!("{} is pulling the power switch!", &name);

        match power.is_on {
            true => msg!("The power is now on."),
            false => msg!("The power is now off!"),
        };

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeLever<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub power: Account<'info, PowerStatus>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetPowerStatus<'info> {
    #[account(mut)]
    pub power: Account<'info, PowerStatus>,
}

#[account]
pub struct PowerStatus {
    pub is_on: bool,
}

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/error.rs
// For any custom errors

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/lib.rs
#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

use crate::instructions::{eat_food, get_on_ride, play_game};

// For setting up modules & configs

declare_id!("8t94SEJh9jVjDwV7cbiuT6BvEsHo4YHP9x9a5rYH1NpP");

#[program]
pub mod carnival {
    use super::*;

    pub fn go_on_ride(
        _ctx: Context<CarnivalContext>,
        name: String,
        height: u32,
        ticket_count: u32,
        ride_name: String,
    ) -> Result<()> {
        get_on_ride::get_on_ride(get_on_ride::GetOnRideInstructionData {
            rider_name: name,
            rider_height: height,
            rider_ticket_count: ticket_count,
            ride: ride_name,
        })
    }

    pub fn play_game(
        _ctx: Context<CarnivalContext>,
        name: String,
        ticket_count: u32,
        game_name: String,
    ) -> Result<()> {
        play_game::play_game(play_game::PlayGameInstructionData {
            gamer_name: name,
            gamer_ticket_count: ticket_count,
            game: game_name,
        })
    }

    pub fn eat_food(
        _ctx: Context<CarnivalContext>,
        name: String,
        ticket_count: u32,
        food_stand_name: String,
    ) -> Result<()> {
        eat_food::eat_food(eat_food::EatFoodInstructionData {
            eater_name: name,
            eater_ticket_count: ticket_count,
            food_stand: food_stand_name,
        })
    }
}

#[derive(Accounts)]
pub struct CarnivalContext<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/state/mod.rs
pub mod food;
pub mod game;
pub mod ride;

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/state/food.rs
// Objects

pub struct FoodStand {
    pub name: String,
    pub food_type: String,
    pub tickets: u32,
}

impl FoodStand {
    pub fn new(name: String, food_type: String, tickets: u32) -> FoodStand {
        FoodStand {
            name,
            food_type,
            tickets,
        }
    }
}

pub fn get_food_stands() -> Vec<FoodStand> {
    vec![
        FoodStand::new("Larry's Pizza".to_string(), "pizza".to_string(), 3),
        FoodStand::new("Taco Shack".to_string(), "taco".to_string(), 2),
        FoodStand::new("Dough Boy's".to_string(), "fried dough".to_string(), 1),
    ]
}

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/state/ride.rs
// Objects

pub struct Ride {
    pub name: String,
    pub upside_down: bool,
    pub tickets: u32,
    pub min_height: u32,
}

impl Ride {
    pub fn new(name: String, upside_down: bool, tickets: u32, min_height: u32) -> Ride {
        Ride {
            name,
            upside_down,
            tickets,
            min_height,
        }
    }
}

pub fn get_rides() -> Vec<Ride> {
    vec![
        Ride::new("Tilt-a-Whirl".to_string(), false, 3, 48),
        Ride::new("Scrambler".to_string(), false, 3, 48),
        Ride::new("Ferris Wheel".to_string(), false, 5, 55),
        Ride::new("Zero Gravity".to_string(), true, 5, 60),
    ]
}

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/state/game.rs
// Objects

pub struct Game {
    pub name: String,
    pub tickets: u32,
    pub tries: u32,
    pub prize: String,
}

const DEFAULT_TICKETS_TO_PLAY: u32 = 3;

impl Game {
    pub fn new(name: String, tries: u32, prize: String) -> Game {
        Game {
            name,
            tickets: DEFAULT_TICKETS_TO_PLAY,
            tries,
            prize,
        }
    }
}

pub fn get_games() -> Vec<Game> {
    vec![
        Game::new("Ring Toss".to_string(), 5, "teddy bear".to_string()),
        Game::new("I Got It!".to_string(), 12, "goldfish".to_string()),
        Game::new("Ladder Climb".to_string(), 1, "popcorn bucket".to_string()),
    ]
}

>>> program-examples/basics/cross-program-invocation/anchor/programs/hand/src/lib.rs
use anchor_lang::prelude::*;

declare_id!("Bi5N7SUQhpGknVcqPTzdFFVueQoxoUu8YTLz75J6fT8A");

// automatically generate module using program idl found in ./idls
declare_program!(lever);
use lever::accounts::PowerStatus;
use lever::cpi::accounts::SwitchPower;
use lever::cpi::switch_power;
use lever::program::Lever;

#[program]
pub mod hand {
    use super::*;

    pub fn pull_lever(ctx: Context<PullLever>, name: String) -> Result<()> {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.lever_program.to_account_info(),
            SwitchPower {
                power: ctx.accounts.power.to_account_info(),
            },
        );
        switch_power(cpi_ctx, name)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct PullLever<'info> {
    #[account(mut)]
    pub power: Account<'info, PowerStatus>,
    pub lever_program: Program<'info, Lever>,
}

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/instructions/get_on_ride.rs
use anchor_lang::prelude::*;

use crate::state::ride;

// Instruction Data

pub struct GetOnRideInstructionData {
    pub rider_name: String,
    pub rider_height: u32,
    pub rider_ticket_count: u32,
    pub ride: String,
}

pub fn get_on_ride(ix: GetOnRideInstructionData) -> Result<()> {
    let rides_list = ride::get_rides();

    for ride in rides_list.iter() {
        if ix.ride.eq(&ride.name) {
            msg!("You're about to ride the {}!", ride.name);

            if ix.rider_ticket_count < ride.tickets {
                msg!(
                    "  Sorry {}, you need {} tickets to ride the {}!",
                    ix.rider_name,
                    ride.tickets,
                    ride.name
                );
                return Ok(());
            };

            if ix.rider_height < ride.min_height {
                msg!(
                    "  Sorry {}, you need to be {}\" tall to ride the {}!",
                    ix.rider_name,
                    ride.min_height,
                    ride.name
                );
                return Ok(());
            };

            msg!("  Welcome aboard the {}!", ride.name);

            if ride.upside_down {
                msg!("  Btw, this ride goes upside down. Hold on tight!");
            };

            return Ok(());
        }
    }

    Err(ProgramError::InvalidInstructionData.into())
}

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/instructions/play_game.rs
use anchor_lang::prelude::*;

use crate::state::game;

// Instruction Data

pub struct PlayGameInstructionData {
    pub gamer_name: String,
    pub gamer_ticket_count: u32,
    pub game: String,
}

pub fn play_game(ix: PlayGameInstructionData) -> Result<()> {
    let games_list = game::get_games();

    for game in games_list.iter() {
        if ix.game.eq(&game.name) {
            msg!("You're about to play {}!", game.name);

            if ix.gamer_ticket_count < game.tickets {
                msg!(
                    "  Sorry {}, you need {} tickets to play {}!",
                    ix.gamer_name,
                    game.tickets,
                    game.name
                );
            } else {
                msg!("  Let's see what you got!");
                msg!(
                    "  You get {} attempts and the prize is a {}!",
                    game.tries,
                    game.prize
                );
            };

            return Ok(());
        }
    }

    Err(ProgramError::InvalidInstructionData.into())
}

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/instructions/mod.rs
pub mod eat_food;
pub mod get_on_ride;
pub mod play_game;

>>> program-examples/basics/repository-layout/anchor/programs/carnival/src/instructions/eat_food.rs
use anchor_lang::prelude::*;

use crate::state::food;

// Instruction Data

pub struct EatFoodInstructionData {
    pub eater_name: String,
    pub eater_ticket_count: u32,
    pub food_stand: String,
}

pub fn eat_food(ix: EatFoodInstructionData) -> Result<()> {
    let food_stands_list = food::get_food_stands();

    for food_stand in food_stands_list.iter() {
        if ix.food_stand.eq(&food_stand.name) {
            msg!("Welcome to {}! What can I get you?", food_stand.name);

            if ix.eater_ticket_count < food_stand.tickets {
                msg!(
                    "  Sorry {}, our {} is {} tickets!",
                    ix.eater_name,
                    food_stand.food_type,
                    food_stand.tickets
                );
            } else {
                msg!("  Enjoy your {}!", food_stand.food_type);
            };

            return Ok(());
        }
    }

    Err(ProgramError::InvalidInstructionData.into())
}

>>> program-examples/tokens/token-2022/transfer-hook/transfer-cost/anchor/programs/transfer-hook/src/lib.rs
use std::{ cell::RefMut, str::FromStr };
use anchor_lang::{ prelude::*, solana_program::pubkey::Pubkey };
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount,
            BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{ transfer_checked, Mint, TokenAccount, TransferChecked },
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    seeds::Seed,
    state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

// transfer-hook program that charges a SOL fee on token transfer
// use a delegate and wrapped SOL because signers from initial transfer are not accessible

declare_id!("FjcHckEgXcBhFmSGai3FRpDLiT6hbpV893n8iTxVd81g");

#[error_code]
pub enum TransferError {
    #[msg("Amount Too big")]
    AmountTooBig,
    #[msg("The token is not currently transferring")]
    IsNotCurrentlyTransferring,
}

#[program]
pub mod transfer_hook {
    use super::*;

    #[interface(spl_transfer_hook_interface::initialize_extra_account_meta_list)]
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>
    ) -> Result<()> {
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas
        )?;

        Ok(())
    }

    #[interface(spl_transfer_hook_interface::execute)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook
        check_is_transferring(&ctx)?;

        if amount > 50 {
            msg!("The amount is too big {0}", amount);
            //return err!(TransferError::AmountTooBig);
        }

        ctx.accounts.counter_account.counter += 1;

        msg!("This token has been transferred {0} times", ctx.accounts.counter_account.counter);

        // All accounts are non writable so you can not burn any of them for example here
        msg!("Is writable mint {0}", ctx.accounts.mint.to_account_info().is_writable);
        msg!(
            "Is destination mint {0}",
            ctx.accounts.destination_token.to_account_info().is_writable
        );
        msg!("Is source mint {0}", ctx.accounts.source_token.to_account_info().is_writable);

        let signer_seeds: &[&[&[u8]]] = &[&[b"delegate", &[ctx.bumps.delegate]]];

        // Transfer WSOL from sender to delegate token account using delegate PDA
        // transfer lamports amount equal to token transfer amount
        transfer_checked(
            CpiContext::new(ctx.accounts.token_program.to_account_info(), TransferChecked {
                from: ctx.accounts.sender_wsol_token_account.to_account_info(),
                mint: ctx.accounts.wsol_mint.to_account_info(),
                to: ctx.accounts.delegate_wsol_token_account.to_account_info(),
                authority: ctx.accounts.delegate.to_account_info(),
            }).with_signer(signer_seeds),
            amount,
            ctx.accounts.wsol_mint.decimals
        )?;
        Ok(())
    }
}

fn check_is_transferring(ctx: &Context<TransferHook>) -> Result<()> {
    let source_token_info = ctx.accounts.source_token.to_account_info();
    let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
    let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
    let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

    if !bool::from(account_extension.transferring) {
        return err!(TransferError::IsNotCurrentlyTransferring);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        )?,
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(init, seeds = [b"counter"], bump, payer = payer, space = 9)]
    pub counter_account: Account<'info, CounterAccount>,
    pub system_program: Program<'info, System>,
}

// Define extra account metas to store on extra_account_meta_list account
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        // When the token2022 program CPIs to the transfer_hook instruction on this program,
        // the accounts are provided in order defined specified the list:

        // index 0-3 are the accounts required for token transfer (source, mint, destination, owner)
        // index 4 is address of ExtraAccountMetaList account
        Ok(
            vec![
                // index 5, wrapped SOL mint
                ExtraAccountMeta::new_with_pubkey(
                    &Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
                    false,
                    false
                )?,
                // index 6, token program (for wsol token transfer)
                ExtraAccountMeta::new_with_pubkey(&Token::id(), false, false)?,
                // index 7, associated token program
                ExtraAccountMeta::new_with_pubkey(&AssociatedToken::id(), false, false)?,
                // index 8, delegate PDA
                ExtraAccountMeta::new_with_seeds(
                    &[
                        Seed::Literal {
                            bytes: b"delegate".to_vec(),
                        },
                    ],
                    false, // is_signer
                    true // is_writable
                )?,
                // index 9, delegate wrapped SOL token account
                ExtraAccountMeta::new_external_pda_with_seeds(
                    7, // associated token program index
                    &[
                        Seed::AccountKey { index: 8 }, // owner index (delegate PDA)
                        Seed::AccountKey { index: 6 }, // token program index
                        Seed::AccountKey { index: 5 }, // wsol mint index
                    ],
                    false, // is_signer
                    true // is_writable
                )?,
                // index 10, sender wrapped SOL token account
                ExtraAccountMeta::new_external_pda_with_seeds(
                    7, // associated token program index
                    &[
                        Seed::AccountKey { index: 3 }, // owner index
                        Seed::AccountKey { index: 6 }, // token program index
                        Seed::AccountKey { index: 5 }, // wsol mint index
                    ],
                    false, // is_signer
                    true // is_writable
                )?,
                ExtraAccountMeta::new_with_seeds(
                    &[
                        Seed::Literal {
                            bytes: b"counter".to_vec(),
                        },
                    ],
                    false, // is_signer
                    true // is_writable
                )?
            ]
        )
    }
}

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint, token::authority = owner)]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(token::mint = mint)]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    pub wsol_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(
        mut,
        seeds = [b"delegate"], 
        bump
    )]
    pub delegate: SystemAccount<'info>,
    #[account(
        mut,
        token::mint = wsol_mint, 
        token::authority = delegate,
    )]
    pub delegate_wsol_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = wsol_mint, 
        token::authority = owner,
    )]
    pub sender_wsol_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(seeds = [b"counter"], bump)]
    pub counter_account: Account<'info, CounterAccount>,
}

#[account]
pub struct CounterAccount {
    counter: u8,
}

>>> program-examples/tokens/token-2022/transfer-hook/hello-world/anchor/programs/transfer-hook/src/lib.rs
use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount,
            BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{
        spl_pod::optional_keys::OptionalNonZeroPubkey,
        spl_token_2022::{
            extension::{
                transfer_hook::TransferHook as TransferHookExtension,
                BaseStateWithExtensions,
                StateWithExtensions,
            },
            state::Mint as MintState,
        },
        Mint,
        Token2022,
        TokenAccount,
    },
};
use spl_tlv_account_resolution::{ account::ExtraAccountMeta, state::ExtraAccountMetaList };
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

declare_id!("jY5DfVksJT8Le38LCaQhz5USeiGu4rUeVSS8QRAMoba");

#[error_code]
pub enum TransferError {
    #[msg("The token is not currently transferring")]
    IsNotCurrentlyTransferring,
}

#[program]
pub mod transfer_hook {
    use super::*;

    // create a mint account that specifies this program as the transfer hook program
    pub fn initialize(ctx: Context<Initialize>, _decimals: u8) -> Result<()> {
        ctx.accounts.check_mint_data()?;
        Ok(())
    }

    #[interface(spl_transfer_hook_interface::initialize_extra_account_meta_list)]
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>
    ) -> Result<()> {
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas
        )?;

        Ok(())
    }

    #[interface(spl_transfer_hook_interface::execute)]
    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook
        check_is_transferring(&ctx)?;

        msg!("Hello Transfer Hook!");

        Ok(())
    }
}

fn check_is_transferring(ctx: &Context<TransferHook>) -> Result<()> {
    let source_token_info = ctx.accounts.source_token.to_account_info();
    let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
    let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
    let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

    if !bool::from(account_extension.transferring) {
        return err!(TransferError::IsNotCurrentlyTransferring);
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(_decimals: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = _decimals,
        mint::authority = payer,
        extensions::transfer_hook::authority = payer,
        extensions::transfer_hook::program_id = crate::ID
    )]
    pub mint_account: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

// helper to check mint data, and demonstrate how to read mint extension data within a program
impl<'info> Initialize<'info> {
    pub fn check_mint_data(&self) -> Result<()> {
        let mint = &self.mint_account.to_account_info();
        let mint_data = mint.data.borrow();
        let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
        let extension_data = mint_with_extension.get_extension::<TransferHookExtension>()?;

        assert_eq!(
            extension_data.authority,
            OptionalNonZeroPubkey::try_from(Some(self.payer.key()))?
        );

        assert_eq!(extension_data.program_id, OptionalNonZeroPubkey::try_from(Some(crate::ID))?);

        msg!("{:?}", extension_data);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        )?,
        payer = payer
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// Define extra account metas to store on extra_account_meta_list account
// In this example there are none
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        Ok(vec![])
    }
}

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint, token::authority = owner)]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(token::mint = mint)]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
}

>>> program-examples/tokens/token-2022/non-transferable/anchor/programs/non-transferable/src/lib.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    token_2022::{
        initialize_mint2,
        spl_token_2022::{extension::ExtensionType, pod::PodMint},
        InitializeMint2,
    },
    token_interface::{non_transferable_mint_initialize, NonTransferableMintInitialize, Token2022},
};

declare_id!("8Bz4wpHaUckiC169Rg5ZfaBHFemp5S8RwTSDTKzhJ9W");

#[program]
pub mod non_transferable {
    use super::*;

    // There is currently not an anchor constraint to automatically initialize the NonTransferable extension
    // We can manually create and initialize the mint account via CPIs in the instruction handler
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        // Calculate space required for mint and extension data
        let mint_size =
            ExtensionType::try_calculate_account_len::<PodMint>(&[ExtensionType::NonTransferable])?;

        // Calculate minimum lamports required for size of mint account with extensions
        let lamports = (Rent::get()?).minimum_balance(mint_size);

        // Invoke System Program to create new account with space for mint and extension data
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            lamports,                          // Lamports
            mint_size as u64,                  // Space
            &ctx.accounts.token_program.key(), // Owner Program
        )?;

        // Initialize the NonTransferable extension
        // This instruction must come before the instruction to initialize the mint data
        non_transferable_mint_initialize(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            NonTransferableMintInitialize {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
            },
        ))?;

        // Initialize the standard mint account data
        initialize_mint2(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                InitializeMint2 {
                    mint: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            2,                               // decimals
            &ctx.accounts.payer.key(),       // mint authority
            Some(&ctx.accounts.payer.key()), // freeze authority
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub mint_account: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

>>> program-examples/tokens/token-2022/transfer-hook/whitelist/anchor/programs/transfer-hook/src/lib.rs
use std::cell::RefMut;

use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount,
            BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut,
        },
        pod::PodAccount,
    },
    token_interface::{ Mint, TokenAccount },
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    seeds::Seed,
    state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

declare_id!("DrWbQtYJGtsoRwzKqAbHKHKsCJJfpysudF39GBVFSxub");

#[error_code]
pub enum TransferError {
    #[msg("The token is not currently transferring")]
    IsNotCurrentlyTransferring,
}

#[program]
pub mod transfer_hook {
    use super::*;

    #[interface(spl_transfer_hook_interface::initialize_extra_account_meta_list)]
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>
    ) -> Result<()> {
        // set authority field on white_list account as payer address
        ctx.accounts.white_list.authority = ctx.accounts.payer.key();

        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas
        )?;
        Ok(())
    }

    #[interface(spl_transfer_hook_interface::execute)]
    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        // Fail this instruction if it is not called from within a transfer hook
        check_is_transferring(&ctx)?;

        if !ctx.accounts.white_list.white_list.contains(&ctx.accounts.destination_token.key()) {
            panic!("Account not in white list!");
        }

        msg!("Account in white list, all good!");

        Ok(())
    }

    pub fn add_to_whitelist(ctx: Context<AddToWhiteList>) -> Result<()> {
        if ctx.accounts.white_list.authority != ctx.accounts.signer.key() {
            panic!("Only the authority can add to the white list!");
        }

        ctx.accounts.white_list.white_list.push(ctx.accounts.new_account.key());
        msg!("New account white listed! {0}", ctx.accounts.new_account.key().to_string());
        msg!("White list length! {0}", ctx.accounts.white_list.white_list.len());

        Ok(())
    }
}

fn check_is_transferring(ctx: &Context<TransferHook>) -> Result<()> {
    let source_token_info = ctx.accounts.source_token.to_account_info();
    let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
    let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
    let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

    if !bool::from(account_extension.transferring) {
        return err!(TransferError::IsNotCurrentlyTransferring);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        )?,
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
    #[account(init_if_needed, seeds = [b"white_list"], bump, payer = payer, space = 400)]
    pub white_list: Account<'info, WhiteList>,
}

// Define extra account metas to store on extra_account_meta_list account
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        Ok(
            vec![
                ExtraAccountMeta::new_with_seeds(
                    &[
                        Seed::Literal {
                            bytes: "white_list".as_bytes().to_vec(),
                        },
                    ],
                    false, // is_signer
                    true // is_writable
                )?
            ]
        )
    }
}

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint, token::authority = owner)]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(token::mint = mint)]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(seeds = [b"white_list"], bump)]
    pub white_list: Account<'info, WhiteList>,
}

#[derive(Accounts)]
pub struct AddToWhiteList<'info> {
    /// CHECK: New account to add to white list
    #[account()]
    pub new_account: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"white_list"],
        bump
    )]
    pub white_list: Account<'info, WhiteList>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[account]
pub struct WhiteList {
    pub authority: Pubkey,
    pub white_list: Vec<Pubkey>,
}

>>> program-examples/tokens/token-2022/transfer-hook/transfer-switch/anchor/programs/transfer-switch/src/state.rs
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct TransferSwitch {
    pub wallet: Pubkey,
    pub on: bool,
}

#[account]
#[derive(InitSpace)]
pub struct AdminConfig {
    pub is_initialised: bool,
    pub admin: Pubkey,
}

>>> program-examples/tokens/token-2022/transfer-hook/transfer-switch/anchor/programs/transfer-switch/src/lib.rs
mod error;
mod instructions;
mod state;

use anchor_lang::prelude::*;
use instructions::*;

declare_id!("FjcHckEgXcBhFmSGai3FRpDLiT6hbpV893n8iTxVd81g");

#[program]
pub mod transfer_switch {
    use super::*;

    pub fn configure_admin(ctx: Context<ConfigureAdmin>) -> Result<()> {
        ctx.accounts.is_admin()?;
        ctx.accounts.configure_admin()
    }

    #[interface(spl_transfer_hook_interface::initialize_extra_account_meta_list)]
    pub fn initialize_extra_account_metas_list(
        ctx: Context<InitializeExtraAccountMetas>,
    ) -> Result<()> {
        ctx.accounts.initialize_extra_account_metas_list(ctx.bumps)
    }

    pub fn switch(ctx: Context<Switch>, on: bool) -> Result<()> {
        ctx.accounts.switch(on)
    }

    #[interface(spl_transfer_hook_interface::execute)]
    pub fn transfer_hook(ctx: Context<TransferHook>) -> Result<()> {
        ctx.accounts.assert_is_transferring()?;
        ctx.accounts.assert_switch_is_on()
    }
}

>>> program-examples/tokens/token-2022/transfer-hook/transfer-switch/anchor/programs/transfer-switch/src/error.rs
use anchor_lang::prelude::*;

#[error_code]
pub enum TransferError {
    #[msg("The token is not currently transferring")]
    IsNotCurrentlyTransferring,

    #[msg("The transfer switch is currently not on")]
    SwitchNotOn,
}

>>> program-examples/tokens/token-2022/interest-bearing/anchor/programs/interest-bearing/src/lib.rs
use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    token_2022::{
        initialize_mint2,
        spl_token_2022::{
            extension::{
                interest_bearing_mint::InterestBearingConfig, BaseStateWithExtensions,
                ExtensionType, StateWithExtensions,
            },
            pod::PodMint,
            state::Mint as MintState,
        },
        InitializeMint2,
    },
    token_interface::{
        interest_bearing_mint_initialize, interest_bearing_mint_update_rate,
        spl_pod::optional_keys::OptionalNonZeroPubkey, InterestBearingMintInitialize,
        InterestBearingMintUpdateRate, Mint, Token2022,
    },
};
declare_id!("DMQdkzRJz8uQSN8Kx2QYmQJn6xLKhsu3LcPYxs314MgC");

#[program]
pub mod interest_bearing {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, rate: i16) -> Result<()> {
        // Calculate space required for mint and extension data
        let mint_size = ExtensionType::try_calculate_account_len::<PodMint>(&[
            ExtensionType::InterestBearingConfig,
        ])?;

        // Calculate minimum lamports required for size of mint account with extensions
        let lamports = (Rent::get()?).minimum_balance(mint_size);

        // Invoke System Program to create new account with space for mint and extension data
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            lamports,                          // Lamports
            mint_size as u64,                  // Space
            &ctx.accounts.token_program.key(), // Owner Program
        )?;

        // Initialize the InterestBearingConfig extension
        // This instruction must come before the instruction to initialize the mint data
        interest_bearing_mint_initialize(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                InterestBearingMintInitialize {
                    token_program_id: ctx.accounts.token_program.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            Some(ctx.accounts.payer.key()),
            rate,
        )?;

        // Initialize the standard mint account data
        initialize_mint2(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                InitializeMint2 {
                    mint: ctx.accounts.mint_account.to_account_info(),
                },
            ),
            2,                               // decimals
            &ctx.accounts.payer.key(),       // mint authority
            Some(&ctx.accounts.payer.key()), // freeze authority
        )?;

        check_mint_data(
            &ctx.accounts.mint_account.to_account_info(),
            &ctx.accounts.payer.key(),
        )?;
        Ok(())
    }

    pub fn update_rate(ctx: Context<UpdateRate>, rate: i16) -> Result<()> {
        interest_bearing_mint_update_rate(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                InterestBearingMintUpdateRate {
                    token_program_id: ctx.accounts.token_program.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    rate_authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            rate,
        )?;

        check_mint_data(
            &ctx.accounts.mint_account.to_account_info(),
            &ctx.accounts.authority.key(),
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub mint_account: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRate<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub mint_account: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

fn check_mint_data(mint_account_info: &AccountInfo, authority_key: &Pubkey) -> Result<()> {
    let mint_data = mint_account_info.data.borrow();
    let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
    let extension_data = mint_with_extension.get_extension::<InterestBearingConfig>()?;

    assert_eq!(
        extension_data.rate_authority,
        OptionalNonZeroPubkey::try_from(Some(*authority_key))?
    );

    msg!("{:?}", extension_data);
    Ok(())
}

>>> program-examples/tokens/token-2022/transfer-hook/transfer-switch/anchor/programs/transfer-switch/src/instructions/mod.rs
pub mod configure_admin;
pub mod initialise_extra_account_metas_list;
pub mod switch;
pub mod transfer_hook;

pub use configure_admin::*;
pub use initialise_extra_account_metas_list::*;
pub use switch::*;
pub use transfer_hook::*;

>>> program-examples/tokens/token-2022/transfer-hook/transfer-switch/anchor/programs/transfer-switch/src/instructions/transfer_hook.rs
use {
    crate::{error::TransferError, state::TransferSwitch},
    anchor_lang::prelude::*,
    anchor_spl::{
        token_2022::spl_token_2022::{
            extension::{
                transfer_hook::TransferHookAccount, BaseStateWithExtensionsMut,
                PodStateWithExtensionsMut,
            },
            pod::PodAccount,
        },
        token_interface::Mint,
    },
};

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct TransferHook<'info> {
    /// CHECK: Sender token account
    #[account()]
    pub source_token_account: UncheckedAccount<'info>,

    /// The mint of the token transferring
    #[account()]
    pub token_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Recipient token account
    #[account()]
    pub receiver_token_account: UncheckedAccount<'info>,

    /// CHECK: the transfer sender
    #[account()]
    pub wallet: UncheckedAccount<'info>,

    /// CHECK: extra account metas
    #[account(
        seeds = [b"extra-account-metas", token_mint.key().as_ref()],
        bump,
    )]
    pub extra_account_metas_list: UncheckedAccount<'info>,

    /// sender transfer switch
    #[account(
        seeds=[wallet.key().as_ref()],
        bump,
    )]
    pub wallet_switch: Account<'info, TransferSwitch>,
}

impl<'info> TransferHook<'info> {
    pub fn assert_switch_is_on(&mut self) -> Result<()> {
        if !self.wallet_switch.on {
            return err!(TransferError::SwitchNotOn);
        }
        Ok(())
    }

    pub fn assert_is_transferring(&self) -> Result<()> {
        let source_token_info = self.source_token_account.to_account_info();
        let mut account_data_ref = source_token_info.try_borrow_mut_data()?;
        let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

        if !bool::from(account_extension.transferring) {
            return err!(TransferError::IsNotCurrentlyTransferring);
        }

        Ok(())
    }
}

>>> program-examples/tokens/token-2022/transfer-hook/transfer-switch/anchor/programs/transfer-switch/src/instructions/configure_admin.rs
use {crate::state::AdminConfig, anchor_lang::prelude::*};

#[derive(Accounts)]
pub struct ConfigureAdmin<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: the new admin
    #[account(mut)]
    pub new_admin: UncheckedAccount<'info>,

    /// To hold the address of the admin that controls switches
    #[account(
        init_if_needed,
        payer=admin,
        space=8+AdminConfig::INIT_SPACE,
        seeds=[b"admin-config"],
        bump
    )]
    pub admin_config: Account<'info, AdminConfig>,

    pub system_program: Program<'info, System>,
}

impl<'info> ConfigureAdmin<'info> {
    pub fn is_admin(&self) -> Result<()> {
        // check if we are not creating the account for the first time,
        // ensure it's the admin that is making the change
        //
        if self.admin_config.is_initialised {
            // make sure it's the admin
            //
            require_keys_eq!(self.admin.key(), self.admin_config.admin,);

            // make sure the admin is not reentering their key
            //
            require_keys_neq!(self.admin.key(), self.new_admin.key());
        }
        Ok(())
    }

    pub fn configure_admin(&mut self) -> Result<()> {
        self.admin_config.set_inner(AdminConfig {
            admin: self.new_admin.key(), // set the admin pubkey that can switch transfers on/off
            is_initialised: true,        // let us know an admin has been set
        });
        Ok(())
    }
}

>>> program-examples/tokens/token-2022/transfer-hook/transfer-switch/anchor/programs/transfer-switch/src/instructions/switch.rs
use {
    crate::state::{AdminConfig, TransferSwitch},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct Switch<'info> {
    /// admin that controls the switch
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: wallet - transfer sender
    #[account(mut)]
    pub wallet: UncheckedAccount<'info>,

    /// admin config
    #[account(
        has_one=admin,
        seeds=[b"admin-config"],
        bump,
    )]
    pub admin_config: Account<'info, AdminConfig>,

    /// the wallet (sender) transfer switch
    #[account(
        init_if_needed,
        payer=admin,
        space=8+TransferSwitch::INIT_SPACE,
        seeds=[wallet.key().as_ref()],
        bump,
    )]
    pub wallet_switch: Account<'info, TransferSwitch>,

    pub system_program: Program<'info, System>,
}

impl<'info> Switch<'info> {
    pub fn switch(&mut self, on: bool) -> Result<()> {
        // toggle switch on/off for the given wallet
        //
        self.wallet_switch.set_inner(TransferSwitch {
            wallet: self.wallet.key(),
            on,
        });
        Ok(())
    }
}

>>> program-examples/tokens/token-2022/transfer-hook/transfer-switch/anchor/programs/transfer-switch/src/instructions/initialise_extra_account_metas_list.rs
use {
    anchor_lang::{
        prelude::*,
        system_program::{create_account, CreateAccount},
    },
    anchor_spl::token_interface::Mint,
    spl_tlv_account_resolution::{
        account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
    },
    spl_transfer_hook_interface::instruction::ExecuteInstruction,
};

#[derive(Accounts)]
pub struct InitializeExtraAccountMetas<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account()]
    pub token_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: extra accoumt metas list
    #[account(
        mut,
        seeds = [b"extra-account-metas", token_mint.key().as_ref()],
        bump,
    )]
    pub extra_account_metas_list: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitializeExtraAccountMetas<'info> {
    pub fn initialize_extra_account_metas_list(
        &self,
        bumps: InitializeExtraAccountMetasBumps,
    ) -> Result<()> {
        let account_metas = vec![
            // 5 - wallet (sender) config account
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::AccountKey { index: 3 }, // sender index
                ],
                false, // is_signer
                false, // is_writable
            )?,
        ];

        // calculate account size
        let account_size = ExtraAccountMetaList::size_of(account_metas.len())? as u64;

        // calculate minimum required lamports
        let lamports = Rent::get()?.minimum_balance(account_size as usize);

        let mint = self.token_mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            mint.as_ref(),
            &[bumps.extra_account_metas_list],
        ]];

        create_account(
            CpiContext::new(
                self.system_program.to_account_info(),
                CreateAccount {
                    from: self.payer.to_account_info(),
                    to: self.extra_account_metas_list.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            lamports,
            account_size,
            &crate::ID,
        )?;

        // Initialize the account data to store the list of ExtraAccountMetas
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut self.extra_account_metas_list.try_borrow_mut_data()?,
            &account_metas,
        )?;

        Ok(())
    }
}

</PART3>
# System Prompt Generator for Solana Rust Code Analysis

## Instructions for Processing Raw Concatenated Files

You are tasked with processing raw concatenated files containing Solana documentation and code examples to create a comprehensive system prompt. Your output will be used to configure an LLM as a specialized Solana program Rust code analyst.

## Critical Requirements

**STRUCTURE OUTPUT FOR TOKEN EFFICIENCY AND LLM INTERPRETATION**

**USE AS MANY TOKENS AS NECESSARY TO PRESERVE ALL RELEVANT CONTEXT**

## Processing Instructions

1. **Extract Only Rust-Related Content**: Filter out non-Rust content while preserving all Rust code, documentation, and related explanations.

2. **Organize Content Hierarchically**: Structure the information logically while maintaining all details.

## Output Format

Create a comprehensive system prompt with the following structure:

```
# SYSTEM PROMPT: Solana Rust Program Code Analyst

You are a highly specialized Solana blockchain program analyst with deep expertise in Rust programming and Solana's account model, program architecture, and development patterns.

## Your Role and Capabilities

[Detailed role description based on the processed content]

## Core Solana Concepts You Must Understand

### Account Model
[Complete documentation about Solana's account model with all technical details]

### [Additional core concepts with complete explanations]

### Anchor Development Model
[Complete documentation about how Solana programs are developed with Anchor ]

### [Additional Anchor concepts with complete explanations]

## Rust Code Patterns and Examples

### Complete Code Examples
[Create a code example for each discrete aspect of Anchor-based solana programming detected from the provided examples]
[The examples should be extensively annotated with comments using the documentation provided in parts 1 & 2]

```

## Final Output Goal

The resulting system prompt should be comprehensive enough that an LLM configured with it can:
- Analyze any Solana Rust program with deep understanding
- Provide detailed explanations of program architecture
- Identify common and uncommon development patterns

**Remember: Your output should be as long as necessary to preserve all relevant information. Total token count is not a constraint - completeness is the only requirement.**
