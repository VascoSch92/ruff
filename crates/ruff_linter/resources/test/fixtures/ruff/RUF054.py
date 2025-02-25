############# Warning ############
# This file contains form feeds. #
############# Warning ############


# These should raise a violation as the form feed(s) come
# after a whitespace or a tab

 
		
 

def _():
    pass

if False:
    print('T')

if True:
   print("True")



# These should not raise an error as the form feed(s)
# come at the beginning of a new line, or not a whitespace
# or a tab, or in a logical line




def _():\
        print("true")


if True:
    print("true"), print("true")


if True:\
    print("!")
