# The One Billion Row Challenge
- Challenge blog post: https://www.morling.dev/blog/one-billion-row-challenge/
- Challenge repository: https://github.com/gunnarmorling/1brc

## Test Hardware
The testing of this code was performed on a Framework 13" Laptop.
| Component | Value |
|:---|:---|
| CPU | AMD Ryzen 7 7840U |
| L1d/i | 256 KiB/core |
| L2 | 8 MiB/core |
| L3 | 16 MiB shared|
| RAM | 64 GB DDR4 |

On this hardware and compiled with the native flag, the run time via criterion is **3.6963 seconds**.

## Optimization Strategy
As stated before, the memmap was used to access the binary of the file. This is likely the single most impactful thing done to process the data.

### Memmap2
Instead of traditional I/O, the input file is mapped directly into the process's virtual address space. This avoids the overhead of copying data from kernel space to user space, allowing the CPU to access file data as if it were loaded into RAM.

### Memchr
To handle high-speed character searching (specifically finding newlines), I utilize the memchr crate. This leverages architecture-specific SIMD (Single Instruction, Multiple Data) instructions, which is significantly more efficient than manual byte-by-byte iteration.

### Chunking
Since I used the memmap crate and not the buffered read in the standard crate, I had to chunk the data myself. Doing this required splitting the file into chunks of an assumed cache size (I assumed 512 Kb). This would potentially leave partial lines split between chunks, necessitating areas of overlap to ensure each line exists in at least one of the chunks.

From the challenge description, the maximum line length is 105 bytes. To handle this, a 128 byte overlap region was made where once the file is split, the start/end point in each file are shifted to the next newline character. The chunks are calculated such that the maxmium size of a chunk is 512 Kb and most are slightly smaller than the full 512 Kb.

### Parsing Numerics
The strategy for parsing and storing the numbers relied on using integers. By storing the data as integers and displaying them as floats, the faster integer addition/subtractino and comparison can be utilized, while taking advantage of the more optimized float division. When parsing the file, each value was essentially multiplied by 10 to remove the decimal point, given the rules that each temperature had only one numebr after the decimal point. Prior to displaying any values, the number is converted to a float and divided by 10.0 to convert to the true value.

### Hashmap
The standard libraries hashmap was utilized even though I am sure a custom implementation would be better.

### Multithreading
The final optimization was multithreading. Since the work queue was formed for chunking, threading across the chunks is relatively seamless. Each thread develops its own hashmap, which is then joined at the end. It may be worth investigating a single hasmap which is locked and altered rather than joining at the end.
