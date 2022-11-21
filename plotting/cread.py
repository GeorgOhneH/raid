import matplotlib.pyplot as plt
import numpy as np
import os
import json


fig, ax = plt.subplots(figsize=(5, 4))
single_x = []
single_y = []
dist_x = []
dist_y = []

file_path = os.path.join(os.path.dirname(__file__), "bigger.txt")
with open(file_path) as f:
    RESULTS = f.read()

RESULTS = RESULTS.replace("\\", "")
for line in RESULTS.splitlines():
    r = json.loads(line.strip())
    if r["reason"] != "benchmark-complete":
        continue
    id = r["id"]
    if id[:5] != "cread":
        continue
    d = int(id[5])
    c = int(id[6])
    if id.split("/")[1] == "single_31509708":
        single_x.append(c)
        single_y.append(r["typical"])
    elif id.split("/")[1] == "dist_31509708":
        dist_x.append(c)
        dist_y.append(r["typical"])


scale = 1e-6
ax.plot(single_x, [y["estimate"]*scale for y in single_y], '-', label=f"controller")
ax.fill_between(single_x, [y["lower_bound"]*scale for y in single_y], [y["upper_bound"]*scale for y in single_y], alpha=0.2)

ax.plot(dist_x, [y["estimate"]*scale for y in dist_y], '-', label=f"checkpoint")
ax.fill_between(dist_x, [y["lower_bound"]*scale for y in dist_y], [y["upper_bound"]*scale for y in dist_y], alpha=0.2)

ax.legend()
ax.set_xlabel('number of checksum devices')
ax.set_ylabel('read time in ms')


file_path = os.path.join(os.path.dirname(__file__), "plots", "cread.pdf")
fig.savefig(file_path,bbox_inches='tight')
plt.show(block=True)