# rust ext4 crate no std

# read example

```sh
git checkout dev
python3 gen_test_files.py
sh gen_img.sh
cargo run
```

# write example (code is in the process of refactoring)

```sh
git checkout write_dev
sh gen_write_img.sh
cargo test test_write
```





