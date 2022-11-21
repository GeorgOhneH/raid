import matplotlib.pyplot as plt
import numpy as np
import os
import json

file_path = os.path.join(os.path.dirname(__file__), "small.txt")
with open(file_path) as f:
    RESULTS = f.read()

RESULTS = RESULTS.replace("\\", "")

color_names = [
    ("blue", "darkblue"),
     ("lightgreen", "darkgreen"),
      ("violet", "darkviolet"),
       ("teal", "#00F0F0"), ("gold", "goldenrod"), ("red", "darkred")]

fig, ax = plt.subplots(figsize=(10, 6))
for i in reversed(range(1, 7)):
    single_x = []
    single_y = []
    dist_x = []
    dist_y = []
    for line in RESULTS.splitlines():
        r = json.loads(line.strip())
        if r["reason"] != "benchmark-complete":
            continue
        id = r["id"]
        
        if id[:7] != "recover":
            continue
        d = int(id[7])
        c = int(id[8])
        f = int(id[9])
        if f != i:
            continue
        if id.split("/")[1] == "single recover":
            single_x.append(c)
            single_y.append(r["typical"])
        elif id.split("/")[1] == "distributed recover":
            dist_x.append(c)
            dist_y.append(r["typical"])


    if len(single_x) == 1:
        single_x.append(5.9)
        single_y = single_y + single_y
        dist_x.append(5.9)
        dist_y = dist_y + dist_y
    scale = 1e-9
    ax.plot(single_x, [y["estimate"]*scale for y in single_y], '-', label=f"controller, {i} failures", color=color_names[i-1][0])
    ax.fill_between(single_x, [y["lower_bound"]*scale for y in single_y], [y["upper_bound"]*scale for y in single_y], alpha=0.2, color=color_names[i-1][0])

    ax.plot(dist_x, [y["estimate"]*scale for y in dist_y], '-', label=f"checkpoint, {i} failures", color=color_names[i-1][1])
    ax.fill_between(dist_x, [y["lower_bound"]*scale for y in dist_y], [y["upper_bound"]*scale for y in dist_y], alpha=0.2, color=color_names[i-1][1])

ax.legend()
ax.set_xlabel('number of checksum devices')
ax.set_ylabel('recover time in seconds')


file_path = os.path.join(os.path.dirname(__file__), "plots", "crecover.pdf")
fig.savefig(file_path,bbox_inches='tight')
plt.show(block=True)