import unittest
import waglayla

class TestLib(unittest.TestCase):

    def test_sompi(self):
        self.assertEqual(waglayla.to_sompi(0.5), 50000000)
        self.assertEqual(waglayla.from_sompi(5000000), 0.05)

if __name__ == "__main__":
    unittest.main()