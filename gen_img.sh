rm -rf ex4.img
dd if=/dev/zero of=ex4.img bs=1M count=1024
mkfs.ext4 ./ex4.img
mkdir tmp
mount ./ex4.img ./tmp/
cd tmp
mkdir -p test_files
cp ../test_files2/* ./
cp ../test_files/* ./test_files/
cp ../test_files/1.txt ./
cp ../test_files/2.txt ./
mkdir -p dirtest1/dirtest2/dirtest3/dirtest4/dirtest5/
cp ../test_files2/* ./dirtest1/dirtest2/dirtest3/dirtest4/dirtest5/
echo BBBBBBBBBBBBBBBBB > 2
cd ../
umount tmp