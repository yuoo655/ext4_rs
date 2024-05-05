import os

if not os.path.exists("test_files"):
    os.mkdir("test_files")

for i in range(2):
    name = "test_files/"+ str(i) + ".txt"
    f = open(name, "w")
    f.write(str(i)  * 200000000)
    f.close()