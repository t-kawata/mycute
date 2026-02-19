package metacognition

import "strings"

// extractJSON extracts the JSON content from a string that might contain markdown code blocks.
func extractJSON(s string) string {
	s = strings.ReplaceAll(s, "```json", "")
	s = strings.ReplaceAll(s, "```", "")
	s = strings.TrimSpace(s)
	start := strings.Index(s, "{")
	end := strings.LastIndex(s, "}")
	if start == -1 || end == -1 || start > end {
		return "{}" // Return empty JSON object if extraction fails
	}
	return s[start : end+1]
}
