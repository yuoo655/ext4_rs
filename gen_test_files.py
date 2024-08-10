import os

if not os.path.exists("test_files"):
    os.mkdir("test_files")

if not os.path.exists("tmp"):
    os.mkdir("tmp")

for i in range(2):
    name = "test_files/"+ str(i) + ".txt"
    f = open(name, "w")
    # 1024M
    f.write(str(i)  * 0x100000 * 1024)

name = "test_files/file_to_remove"
f = open(name, "w")
#1MB * 1024
f.write("A"  * (0x100000 * 1024))
f.close()