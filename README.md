# Lnkx

A simple tool made in rust to open windows shell links (.lnk) on Linux by just double-clicking

Features:

- [x] Reads .lnk files
- [x] Converts .lnk files to corresponding path on linux
- [x] Runs the windows program using wine
- [x] Runs the program with arguments using wine
- [x] Open .url files
- [ ] Runs Linux binaries with args
- [ ] Use it as a CLI to create .lnk files for linux binaries
- [ ] Use it as a CLI to convert .lnk files to symlinks and viceversa

# How to build

```
> Install rust and cargo
git clone https://github.com/davidevofficial/lnkx.git
cd lnkx
cargo build --release
sudo chmod +x ./target/release/lnkx
sudo cp ./target/release/lnkx /usr/local/bin/lnkx # (optional) 
> Set lnkx as the default program for opening .lnk files
```

# Contributions

Any contributions are welcome. For feedback/suggestions send an email to davidevufficial@gmail.com 

# License

lnkx Source-Available License

Copyright (c) 2026 davidevofficial

Permission is granted to view, compile, run, and modify this software
for personal, educational, and non-commercial purposes.

You may not distribute, publish, sublicense, sell, rent, or offer
this software, modified versions of it, or works based on it,
whether for payment or otherwise.

Commercial use, including use in a product, service, business,
paid tool, or revenue-generating project, is prohibited without
prior written permission from the copyright holder.

Modified versions must remain private and may not be redistributed.

If you are unsure whether a particular use, modification,
distribution, or other activity is permitted under this license,
contact the copyright holder by email before proceeding:
davidevufficial@gmail.com

Permission for any activity not explicitly granted by this license
must be obtained in writing from the copyright holder.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.
