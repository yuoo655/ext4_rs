rm -rf ex4.img
dd if=/dev/zero of=ex4.img bs=1M count=1024
mkfs.ext4 ./ex4.img
mount ./ex4.img ./tmp/
cd tmp
mkdir -p test_files
cp ../test_files/* ./test_files/
cd ../
umount tmp
cargo run