import zlib

from Crypto.Cipher import AES


def unpack(file_path):
    key = b"1289@Patcher0000"
    iv = b"PatcherSDK000000"
    with open(file_path, "rb") as f:
        data = f.read()
    payload = data[16:]
    cipher = AES.new(key, AES.MODE_CBC, iv)
    dec = cipher.decrypt(payload)
    save(file_path, dec, "zlib")
    decompressor = zlib.decompressobj()
    xml = decompressor.decompress(dec)
    save(file_path, xml, "xml")


def save(original_path, final_data, file_ext):
    out_path = original_path + "." + file_ext
    with open(out_path, "wb") as f:
        f.write(final_data)


if __name__ == "__main__":
    unpack("ResList.bin")
