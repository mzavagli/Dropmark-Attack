import math

def calculate_minimum_id_length(N):
	L = math.log2((N * (N - 1)) / 0.002)
	return L


N = int(input('N>'))
print(calculate_minimum_id_length(N))
