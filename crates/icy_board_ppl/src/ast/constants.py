f = open("constants.dat", "r")
print("------------------")
f = open("constants.dat", "r")

categories = {}

for line in f:
	line = line.strip()
	if line == "":
		continue
	split = line.split(",")	
	v = split[2].strip().split("+")	
	for i in range(len(v)):
		categories[v[i]] = 1
		v[i] = "ConstantType::"+v[i] 
	print(" BuiltinConst { name: \""+split[0].strip()+"\", value: "+split[1].strip()+", used_by: &["+','.join(v)+"]},")

print()
for key, cat in categories.items():
	print(key + ",")