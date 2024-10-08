import matplotlib.pyplot as plt
import pandas as pd
from collections import Counter

# Sample list of timestamps
timestamps_with_injection = [
'00:25:00.050',
'00:25:00.250',
'00:25:00.250',
'00:25:00.250',
'00:25:00.350',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.750',
'00:25:00.750',
'00:25:00.750',
'00:25:00.750',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951'
]

timestamps_without_injection = [
'00:25:00.050',
'00:25:00.350',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.450',
'00:25:00.750',
'00:25:00.750',
'00:25:00.750',
'00:25:00.750',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.850',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951',
'00:25:00.951'
]

# Convert timestamps to datetime objects
timestamps = pd.to_datetime(timestamps_with_injection)

# Count the occurrences of each timestamp
timestamp_counts = Counter(timestamps)

# Create a DataFrame from the counts
df = pd.DataFrame.from_dict(timestamp_counts, orient='index').reset_index()
df.columns = ['Timestamp', 'Count']
df = df.sort_values(by='Timestamp')

# Plot the data
plt.figure(figsize=(10, 6))
plt.plot(df['Timestamp'], df['Count'], marker='o')
plt.xlabel('Time')
plt.ylabel('Number of Cells')
plt.title('Number of cells from 25:00.00 to 25:01.00 with dromark signals')
plt.grid(True)
plt.show()
