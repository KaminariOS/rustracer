import os
from pathlib import Path

def get_license(filename):
    return filename == "license.txt"

record = []
for root, dirs, files in os.walk(".", topdown=False):
   for name in filter(get_license, files):
    full_path = os.path.join(root, name)
    path = Path(full_path)
    parent_dir = path.parent.name 
    record.append('Name: ' + str(parent_dir) + '\n')
    with open(full_path) as input_file:
        head = [next(input_file) for _ in range(5)]
    record += head
print(''.join(record))
   # for name in dirs:
      # print(os.path.join(root, name))
