import os

if not os.path.exists("test_files"):
    os.mkdir("test_files")

for i in range(100):
    name = "test_files/"+ str(i) + ".txt"
    f = open(name, "w")
    f.write(str(i)  * 2000)
    f.close()