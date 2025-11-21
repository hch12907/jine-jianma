# 寻找结构性重码的汉字

with open("./scripts/yuchai.txt", "rt") as f:
    yuchai = f.read()

result = []
for line in yuchai.splitlines():
    if not line: continue
    [zi, roots] = line.split('\t')
    roots = roots.split(' ')
    result.append((zi, roots))

result = list(filter(lambda x: ord(x[0]) < 0x9FFF, result))
sorted_roots = sorted(result, key=lambda x: x[1])
isomorphs = []

for i, (zi, roots) in enumerate(sorted_roots):
    if ((i+1 < len(sorted_roots) and roots == sorted_roots[i+1][1]) or
        (i > 0 and roots == sorted_roots[i-1][1])):
        print(zi, sorted_roots[i+1][0], "是结构重码字")
        isomorphs.append((zi, roots))

with open("isomorphs.txt", "wt") as f:
    out = ""
    prev_roots = []
    for zi, roots in isomorphs:
        if prev_roots and prev_roots != roots: out += "\n"
        out += f"{zi}\t{roots}\n"
        prev_roots = roots
    f.write(out)
