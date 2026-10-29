import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest

spec = importlib.util.spec_from_file_location('harness', str(Path(__file__).with_name('test_stdlib_extern_spec.py')))
harness = importlib.util.module_from_spec(spec)
spec.loader.exec_module(harness)

class Watchdog(unittest.TestCase):
    def test_normal_exit_and_output(self):
        code, timed_out, output = harness.run_owned([sys.executable, '-c', 'print("done")'], '/', os.environ.copy(), 2)
        self.assertEqual((code, timed_out, output), (0, False, 'done\n'))

    def test_timeout_kills_observed_grandchild_without_touching_unrelated(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / 'pid'
            program = 'import subprocess,sys,time; p=subprocess.Popen([sys.executable,"-c","import time; time.sleep(30)"]); open(sys.argv[1],"w").write(str(p.pid)); time.sleep(30)'
            unrelated = __import__('subprocess').Popen([sys.executable, '-c', 'import time; time.sleep(30)'])
            try:
                code, timeout, output = harness.run_owned([sys.executable, '-c', program, str(pidfile)], '/', os.environ.copy(), 0.4)
                self.assertTrue(timeout)
                self.assertNotEqual(code, 0)
                self.assertIn('exceeded', output)
                child = int(pidfile.read_text())
                stat = Path(f'/proc/{child}/stat')
                self.assertFalse(stat.exists(), 'tracked grandchild was not reaped')
                self.assertIsNone(unrelated.poll())
            finally:
                unrelated.kill()
                unrelated.wait()

if __name__ == '__main__': unittest.main()
