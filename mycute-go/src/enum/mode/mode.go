package mode

import (
	"fmt"
	"strings"

	"github.com/t-kawata/mycute/config"
	"github.com/thoas/go-funk"
)

type mode struct {
	value       string
	description string
}

var (
	RT = mode{value: "rt", description: "Run as REST API server."}
	AM = mode{value: "am", description: "Run auto migration for db."}
)

func (m *mode) Val() string {
	return m.value
}

func List() []mode {
	return []mode{RT, AM}
}

func Help() string {
	rtn := []string{fmt.Sprintf("[Available Modes] %s", config.VERSION)}
	for _, m := range List() {
		rtn = append(rtn, fmt.Sprintf("> %s\n\t%s", m.value, m.description))
	}
	return strings.Join(rtn, "\n") + "\n"
}

func IsValidMode(m *string) bool {
	f := funk.Filter(List(), func(md mode) bool {
		return md.value == *m
	})
	return len(f.([]mode)) > 0
}
