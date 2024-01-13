import socket

server = socket.socket(
    socket.AF_BLUETOOTH,
    socket.SOCK_STREAM,
    socket.BTPROTO_RFCOMM)
server.bind(('B8:27:EB:C7:5B:1D', 1))
server.listen(1)

print('Starting server')
client, addr = server.accept()
print('Accepted client')

try:
    with open('/dev/hidg0', 'wb') as device:
        while True:
            data = client.recv(7)
            if data:
                print(f'Message: {data}')
                device.write(data)
                device.flush()
            else:
                print('Exiting')
                break

except KeyboardInterrupt:
    print('Received Ctrl-C')

finally:
    client.close()
    server.close()
