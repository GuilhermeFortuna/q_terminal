import tempfile
import unittest
from pathlib import Path

import token_gate


class TokenGateTests(unittest.TestCase):
    def test_clean_qml_uses_tokens(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "Clean.qml"
            path.write_text(
                'import QtQuick\nRectangle { width: Theme.controlHeight; color: Theme.surfaceBase }\n',
                encoding="utf-8",
            )
            self.assertEqual(token_gate.scan_paths([path]), [])

    def test_fixture_reports_every_forbidden_category(self) -> None:
        violations = token_gate.scan_paths(
            [Path("tools/fixtures/token_gate_invalid.qml")]
        )
        categories = {violation.category for violation in violations}
        self.assertEqual(
            categories,
            {"color-literal", "pixel-size", "raw-dimension", "money-arithmetic"},
        )


if __name__ == "__main__":
    unittest.main()
