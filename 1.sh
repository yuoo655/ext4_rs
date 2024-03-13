rm -rf ex4.img
dd if=/dev/zero of=ex4.img bs=1M count=2048
mkfs.ext4 ./ex4.img
mkdir tmp
sudo mount ./ex4.img ./tmp/
cd tmp
sudo mkdir -p test_files
sudo cp ../test_files/* ./test_files/
cd ../
sudo umount tmp
cargo run
sudo mount ./ex4.img ./tmp/
cd tmp
ls
cd dirtest0
ls
cd ../../
sudo umount ./tmp
