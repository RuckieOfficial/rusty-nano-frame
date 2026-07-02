from PIL import Image
import lz4.block
import struct

img = Image.open('/Users/ruckie/Pictures/portalmin.gif')
out = open('anim.lz4', 'wb')

frames_processed = 0
total_size = 0

for i in range(img.n_frames):
    img.seek(i)
    frame_rgb = img.convert("RGB").resize((240, 280), Image.Resampling.LANCZOS)
    pixels = frame_rgb.load()
    
    raw_bytes = bytearray()
    for y in range(280):
        for x in range(240):
            r, g, b = pixels[x, y]
            c = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)
            raw_bytes.extend(c.to_bytes(2, byteorder='big'))
            
    # Compress raw block
    compressed = lz4.block.compress(bytes(raw_bytes), store_size=False)
    
    # Write 4-byte LE size of compressed chunk
    out.write(struct.pack('<I', len(compressed)))
    # Write compressed data
    out.write(compressed)
    
    total_size += 4 + len(compressed)
    frames_processed += 1

out.close()
print(f"Generated anim.lz4: {frames_processed} frames, {total_size} bytes")
