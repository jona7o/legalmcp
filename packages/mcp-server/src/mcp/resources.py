"""MCP Resources - Static resources provided via MCP protocol."""

from typing import Any, Dict


class MCPResources:
    """Static resources for MCP protocol."""

    @staticmethod
    def get_jurisdictions() -> Dict[str, Any]:
        """
        Get available jurisdictions.

        Returns:
            Dictionary with jurisdiction information
        """
        return {
            "jurisdictions": ["federal", "eu", "bavaria", "other"],
            "descriptions": {
                "federal": "German Federal Law (Bundesrecht)",
                "eu": "European Union Law",
                "bavaria": "Bavarian State Law (Bayerisches Landesrecht)",
                "other": "Other jurisdictions",
            },
        }

    @staticmethod
    def get_document_types() -> Dict[str, Any]:
        """
        Get available document types.

        Returns:
            Dictionary with document type information
        """
        return {
            "document_types": ["law", "regulation", "directive", "decision", "other"],
            "descriptions": {
                "law": "Legislative acts (Gesetze)",
                "regulation": "Regulatory acts (Verordnungen)",
                "directive": "EU Directives",
                "decision": "EU Decisions",
                "other": "Other document types",
            },
        }

    @staticmethod
    def get_court_levels() -> Dict[str, Any]:
        """
        Get available court levels.

        Returns:
            Dictionary with court level information
        """
        return {
            "court_levels": ["bgh", "bverwg", "bfh", "bsg", "bag", "lg", "ag", "vg", "other"],
            "descriptions": {
                "bgh": "Bundesgerichtshof (Federal Court of Justice)",
                "bverwg": "Bundesverwaltungsgericht (Federal Administrative Court)",
                "bfh": "Bundesfinanzhof (Federal Fiscal Court)",
                "bsg": "Bundessozialgericht (Federal Social Court)",
                "bag": "Bundesarbeitsgericht (Federal Labour Court)",
                "lg": "Landgericht (Regional Court)",
                "ag": "Amtsgericht (Local Court)",
                "vg": "Verwaltungsgericht (Administrative Court)",
                "other": "Other courts",
            },
        }
