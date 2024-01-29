rm -rf ex4.img
dd if=/dev/zero of=ex4.img bs=1M count=1024
mkfs.ext4 ./ex4.img