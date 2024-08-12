rm -rf ex4.img
dd if=/dev/zero of=ex4.img bs=1M count=8192
mkfs.ext4 ./ex4.img
rm -rf tmp
mkdir tmp
mount ./ex4.img ./tmp/
cd tmp
mkdir -p test_files
cp ../test_files/* ./test_files/
mkdir -p dirtest1/dirtest2/dirtest3/dirtest4/dirtest5/
cp ../test_files/* ./dirtest1/dirtest2/dirtest3/dirtest4/dirtest5/
cd ../
umount tmp