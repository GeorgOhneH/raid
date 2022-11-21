import matplotlib.pyplot as plt
import numpy as np
import os
import json


file_path = os.path.join(os.path.dirname(__file__), "bigger.txt")
with open(file_path) as f:
    RESULTS = f.read()

RESULTS = RESULTS.replace("\\", "")

fig, ax = plt.subplots(figsize=(5, 4))
for i in range(1, 3):
    single_x = []
    single_y = []
    dist_x = []
    dist_y = []
    for line in RESULTS.splitlines():
        r = json.loads(line.strip())
        if r["reason"] != "benchmark-complete":
            continue
        id = r["id"]
        
        if id[:8] != "drecover":
            continue
        _, d, c, f = id.split("/")[0].split("_")
        d = int(d)
        c = int(c)
        f = int(f)
        if f != i:
            continue
        if id.split("/")[1] == "single recover":
            single_x.append(d)
            single_y.append(r["typical"])
        elif id.split("/")[1] == "distributed recover":
            dist_x.append(d)
            dist_y.append(r["typical"])


    scale = 1e-9
    
    ax.plot(single_x, [y["estimate"]*scale for y in single_y], '-', label=f"controller, {i} failures", alpha=0.5)
    ax.fill_between(single_x, [y["lower_bound"]*scale for y in single_y], [y["upper_bound"]*scale for y in single_y], alpha=0.2)

    ax.plot(dist_x, [y["estimate"]*scale for y in dist_y], '-', label=f"checkpoint, {i} failures")
    ax.fill_between(dist_x, [y["lower_bound"]*scale for y in dist_y], [y["upper_bound"]*scale for y in dist_y], alpha=0.2)

ax.legend()
ax.set_xlabel('number of data devices')
ax.set_ylabel('recover time in seconds')


file_path = os.path.join(os.path.dirname(__file__), "plots", "drecover.pdf")
fig.savefig(file_path,bbox_inches='tight')
plt.show(block=True)