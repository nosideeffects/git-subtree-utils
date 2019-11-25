# git-subtree-utils

### Why

Git subtrees are a great alternative to submodules, especially when you want to co-develop multiple repository in sync that may have multiple cross depedencies. However, Git does not provide a mechanism to track these repositories -- `git subtree` is just a fancy merge command.

### Requirements

- Rust v1.39.0 or later
- Patience of a saint

### Get Started
> Note: rustup command assumes you are running a Unix-based operating system. If you are not, visit [rustup.rs](https://rustup.rs) yourself for the correct install command.

```bash
# Install Rust via its toolchain installer -- rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Cargo to $PATH
echo -e '\n# Cargo\nsource $HOME/.cargo/env' >> ~/.bashrc

# Install gitstu
cargo install --path ./gitstu/

# See what it can do
gitstu -h
```
