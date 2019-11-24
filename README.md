# git-subtree-utils

### Requirements

1. Deno v0.24.0 or higher
   ```bash
   # Install nvm equivalent -- asdf
   git clone https://github.com/asdf-vm/asdf.git ~/.asdf --branch v0.7.5
   echo -e '\n. $HOME/.asdf/asdf.sh' >> ~/.bashrc
   echo -e '\n. $HOME/.asdf/completions/asdf.bash' >> ~/.bashrc
   
   # Install Deno
   asdf plugin-add deno https://github.com/asdf-community/asdf-deno.git
   
   asdf install deno 0.24.0
   
   # Activate globally with:
   asdf global deno 0.24.0
   
   # Activate locally in the current folder with:
   asdf local deno 0.24.0
   ```