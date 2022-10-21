"""
Client that actually tests the default server runtime

"""
import socket
from time import sleep
from unittest import TestCase, main


class TestRatelimit(TestCase):
    def setUp(self) -> None:
        super().setUp()
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect(("localhost", 11211))
        self.sock.settimeout(0.01)

        self.addCleanup(self.sock.close)

    def exchange(self, cmd) -> str:
        cmd += '\r\n'
        self.sock.send(cmd.encode())
        return self.sock.recv(10).decode().strip()


    def test_key_diff(self):
        for key in ('foo', 'bar'):
            for _ in range(5):
                cmd = f'incr {key}'
                ret = self.exchange(cmd)

                self.assertEqual(ret, '0')
        
        for key in ('foo', 'bar'):
            cmd = f'incr {key}'
            ret = self.exchange(cmd)

            self.assertEqual(ret, '1')
    
    def test_multiple(self):
        for key in ('foo', 'bar'):
            for _ in range(2):
                ret = self.exchange(f'incr 2/1_{key}')
                self.assertEqual(ret, '0')

                ret = self.exchange(f'incr 3/2_{key}')
                self.assertEqual(ret, '0')

            ret = self.exchange(f'incr 2/1_{key}')
            self.assertEqual(ret, '1')
            
            ret = self.exchange(f'incr 3/2_{key}')
            self.assertEqual(ret, '0')

            ret = self.exchange(f'incr 3/2_{key}')
            self.assertEqual(ret, '1')
        
        sleep(1)

        for key in ('foo', 'bar'):
            ret = self.exchange(f'incr 2/1_{key}')
            self.assertEqual(ret, '0')

            ret = self.exchange(f'incr 3/2_{key}')
            self.assertEqual(ret, '1')
        
        sleep(1)
        for key in ('foo', 'bar'):
            ret = self.exchange(f'incr 3/2_{key}')
            self.assertEqual(ret, '0')




            



def main2():
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(("localhost", 11211))

    for s in ('foo', 'bar'):
        for i in range(5):
            cmd = f'incr {s}'
            sock.send(cmd.encode())
            res = sock.recv(5).decode().strip()
            print(f'{cmd}: {res}')



if __name__ == '__main__':
    main()