## Prerequisites

Before you begin, make sure you have installed the [Solana CLI](https://solana.com/docs/intro/installation).

## Running the Example

Follow these steps to set up a local Solana environment, build and deploy the program, and run the client example.

### 1. Start a Local Validator

- **Launch the Validator**  
  Run the following command to start a local Solana validator:

  ```
  solana-test-validator
  ```

- **Configure the CLI**  
  Point your CLI to the local validator by running:
  ```
  solana config set -ul
  ```

### 2. Build and Deploy the Program

- **Navigate to the Program Directory**  
  Change into the `program` directory:

  ```
  cd program
  ```

- **Build the Program**  
  Compile the program using:

  ```
  cargo build-sbf
  ```

  The build process creates the following files in the `target/deploy` directory:

  - `program.so`: The compiled program.
  - `program-keypair.json`: The keypair file; its public key serves as the program ID.

- **Retrieve the Program ID**  
  Get the program ID by running:

  ```
  solana address -k ./target/deploy/program-keypair.json
  ```

- **Deploy the Program**  
  Deploy your program to the local validator:
  ```
  solana program deploy ./target/deploy/program.so
  ```
  You should see output similar to:
  ```
  Program Id: 94bNzuQWHxdZZVdCmfFTvsVspmrbAu7tgsofD2GkF9LB
  Signature: 3fqKUvcp6tBdeJQMP7upmkN6tphixSZkvfEqwfTjTDtEVjYeDJxAop2s3p4kNLH1Qt2FM86dGwMSJDfDP487PGXv
  ```

### 3. Run the Client

- **Navigate to the Client Directory**  
  Move from the `program` directory to the `client` directory:

  ```
  cd ..
  cd client
  ```

- **Update the Client Code**  
  Open `main.rs` and update the program ID to match your deployed program:

  ```rust
  // Replace with the correct program ID from your deployment
  const PROGRAM_ID: &str = "94bNzuQWHxdZZVdCmfFTvsVspmrbAu7tgsofD2GkF9LB";
  ```

- **Run the Client**  
  Execute the client example to invoke the program:
  ```
  cargo run
  ```
  Expected output (note that your transaction signature will be different):
  ```
  Transaction: https://explorer.solana.com/tx/65GgEpgHF8yhTKGejSoxMYUChfVPYnfeRsyLGoh8zm8e3TMS3Ccvbgat85dxTZMBqExD2BBnnUJG8K5ox7bB2Ds2?cluster=custom
  ```
