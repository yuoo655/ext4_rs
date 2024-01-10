import os

os.mkdir("test_files")
os.mkdir("test_files2")

for i in range(10000):
    name = "test_files/"+ str(i) + ".txt"
    f = open(name, "w")
    f.write("A" * 2000)
    f.close()

for i in range(10000):
    name = "test_files2/"+ str(i) + ".txt"
    f = open(name, "w")
    f.write(str(i) * 2000)
    f.close()