import matplotlib.pyplot as plt
import numpy as np
import os
import json

read_single_x = []
read_single_y = []
read_dist_x = []
read_dist_y = []


write_single_x = []
write_single_y = []
write_dist_x = []
write_dist_y = []


file_path = os.path.join(os.path.dirname(__file__), "small.txt")
with open(file_path) as f:
    RESULTS = f.read()

RESULTS = RESULTS.replace("\\", "")
for line in RESULTS.splitlines():
    r = json.loads(line.strip())
    if r["reason"] != "benchmark-complete":
        continue
    id = r["id"]
    if id[0:4] == "read":
        if id[5:9] == "dist":
            read_dist_x.append(int(id[10:]))
            read_dist_y.append(r["typical"])
        elif id[5:11] == "single":
            x = int(id[12:])
            print(x, r["typical"]["unit"])
            read_single_x.append(int(id[12:]))
            read_single_y.append(r["typical"])
    elif id[0:5] == "write":
        if id[6:10] == "dist":
            write_dist_x.append(int(id[11:]))
            write_dist_y.append(r["typical"])
        elif id[6:12] == "single":
            x = int(id[13:])
            write_single_x.append(int(id[13:]))
            write_single_y.append(r["typical"])

write_single_y = [x for _, x in sorted(zip(write_single_x, write_single_y), key=lambda pair: pair[0])]
write_single_x.sort()
write_dist_y = [x for _, x in sorted(zip(write_dist_x, write_dist_y), key=lambda pair: pair[0])]
write_dist_x.sort()

scale = 1e-6

fig, ax = plt.subplots(figsize=(5, 4))
fig2, ax2 = plt.subplots(figsize=(5, 4))
ax.plot(read_single_x, [y["estimate"]*scale for y in read_single_y], '-', label="controller")
ax.fill_between(read_single_x, [y["lower_bound"]*scale for y in read_single_y], [y["upper_bound"]*scale for y in read_single_y], alpha=0.2)


ax2.plot(write_single_x, [y["estimate"]*scale for y in write_single_y], '-', label="controller")
ax2.fill_between(write_single_x, [y["lower_bound"]*scale for y in write_single_y], [y["upper_bound"]*scale for y in write_single_y], alpha=0.2)

ax.plot(read_dist_x, [y["estimate"]*scale for y in read_dist_y], '-', label="checkpoint")
ax.fill_between(read_dist_x, [y["lower_bound"]*scale for y in read_dist_y], [y["upper_bound"]*scale for y in read_dist_y], alpha=0.2)

ax2.plot(write_dist_x, [y["estimate"]*scale for y in write_dist_y], '-', label="checkpoint")
ax2.fill_between(write_dist_x, [y["lower_bound"]*scale for y in write_dist_y], [y["upper_bound"]*scale for y in write_dist_y], alpha=0.2)

ax.set_xlabel('file size in bytes')
ax.set_ylabel('read time in ms')


ax2.set_xlabel('file size in bytes')
ax2.set_ylabel('write time in ms')

ax.legend()
ax2.legend()


file_path = os.path.join(os.path.dirname(__file__), "plots", "file_read.pdf")
fig.savefig(file_path,bbox_inches='tight')
file_path = os.path.join(os.path.dirname(__file__), "plots", "file_write.pdf")
fig2.savefig(file_path,bbox_inches='tight')
plt.show(block=True)