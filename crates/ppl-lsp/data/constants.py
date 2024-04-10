f = open("FUNCS", "r")
#for line in f:
#	print("hint-statement-"+ line.strip().lower()+"=todo")


for line in f:
	print("        OpCode::" + line.strip() + " => get_hint(fl!(crate::LANGUAGE_LOADER, \"hint-statement-"+ line.strip().lower()+"\")),")
