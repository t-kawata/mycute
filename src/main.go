package main

import (
	_ "embed"
	"fmt"
	"log"
	"os"

	config "github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/enum/mode"
	"github.com/t-kawata/mycute/mode/am"
	"github.com/t-kawata/mycute/mode/rt"
)

func main() {
	if len(os.Args) < 2 {
		log.Fatal("Missing 1st arg as mode to run.")
		return
	}
	m := os.Args[1]
	if m == "-h" || m == "--help" {
		fmt.Println(mode.Help())
		return
	}
	if m == "-v" {
		fmt.Printf("%s\n", config.VERSION)
		return
	}
	switch m {
	case mode.RT.Val():
		mainOfRT()
	case mode.AM.Val():
		am.MainOfAM()
	}
}

// @Title MYCUTE
// @Description ## API概要\nMYCUTE REST APIを定義する。\nURL最大長のリスクを避ける為、検索は query parameter ではなく body json を使用する。\nGin に SEARCH/QUERY method の実装がないため、検索を POST にて行う。
// @Schemes http
// @BasePath /
func mainOfRT() {
	rt.MainOfRT()
}
