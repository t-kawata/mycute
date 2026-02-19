package common

import (
	"flag"
	"fmt"
	"os"
	"time"

	"github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/enum/mode"
	"github.com/t-kawata/mycute/lib/httpclient"
	"github.com/t-kawata/mycute/lib/logger"
	"go.uber.org/zap"
)

type CommonFlags struct {
	Env      string
	LogLevel string
	Output   string
}

type Flag struct {
	Dst     *string
	Name    string
	Default string
	Doc     string
}

func Init(flgName string, flags *[]Flag) (m *string, cflgs *CommonFlags, l *zap.Logger, env *config.Env, hc *httpclient.HttpClient, err error) {
	m = &os.Args[1]
	fs := flag.NewFlagSet(flgName, flag.ExitOnError)
	cflgs = &CommonFlags{}
	for _, f := range *flags {
		fs.StringVar(f.Dst, f.Name, f.Default, f.Doc)
	}
	fs.StringVar(&cflgs.Env, "e", "local", "Environment.")
	fs.StringVar(&cflgs.LogLevel, "l", "debug", "Log Level.")
	fs.StringVar(&cflgs.Output, "o", "stdout", "Destination of log output.")

	err = fs.Parse(os.Args[2:])
	if err != nil {
		err = fmt.Errorf("Failed to parse flags.")
		return
	}

	if *m == "-h" || *m == "--help" {
		fmt.Print(mode.Help())
		return
	}

	if !mode.IsValidMode(m) {
		err = fmt.Errorf("Invalid mode (%s).\n", *m)
		return
	}

	if len(cflgs.Env) == 0 {
		err = fmt.Errorf("Missing -e arg.")
		return
	}

	env = config.GetEnv(cflgs.Env)
	if env.Empty {
		err = fmt.Errorf("Failed to get env data. There is no env named as %s.", cflgs.Env)
		return
	}

	if len(cflgs.LogLevel) == 0 {
		err = fmt.Errorf("Missing -l arg.")
		return
	}

	O := "stdout"
	if cflgs.Output != O {
		if len(cflgs.Output) != 0 {
			O = cflgs.Output
		}
	}

	l = logger.BuildLogger(&cflgs.LogLevel, &O)

	if l == nil {
		err = fmt.Errorf("Invalid log level (%s).", cflgs.LogLevel)
		return
	}

	hc = httpclient.NewDefaultHttpClient(l, env)

	location, err := time.LoadLocation("Asia/Tokyo")
	if err != nil {
		err = fmt.Errorf("Failed to get location.")
		return
	}
	time.Local = location
	return
}
