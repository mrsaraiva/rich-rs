//go:build cgo

package rich

import (
	"os"
	"regexp"
	"sort"
	"strings"
	"testing"
)

func TestCanonicalHeaderSymbolCoverage(t *testing.T) {
	var header []byte
	var err error
	for _, path := range []string{"../../ffi/include/rich.h", "include/rich.h"} {
		header, err = os.ReadFile(path)
		if err == nil {
			break
		}
	}
	if err != nil {
		t.Fatalf("read canonical or packaged rich.h: %v", err)
	}
	re := regexp.MustCompile(`\b(rich_[a-z0-9_]+)\s*\(`)
	set := make(map[string]struct{})
	for _, match := range re.FindAllStringSubmatch(string(header), -1) {
		set[match[1]] = struct{}{}
	}
	headerSymbols := make([]string, 0, len(set))
	for symbol := range set {
		headerSymbols = append(headerSymbols, symbol)
	}
	sort.Strings(headerSymbols)
	inventory := append([]string(nil), nativeSymbolInventory...)
	sort.Strings(inventory)
	if len(headerSymbols) != 113 {
		t.Fatalf("header symbols = %d, want 113", len(headerSymbols))
	}
	if strings.Join(headerSymbols, "\n") != strings.Join(inventory, "\n") {
		t.Fatalf("symbol inventory drift\nheader=%v\ninventory=%v", headerSymbols, inventory)
	}
	bridge, err := os.ReadFile("native_cgo.go")
	if err != nil {
		t.Fatal(err)
	}
	bridgeText := strings.Split(string(bridge), "// This exhaustive inventory")[0]
	for _, symbol := range headerSymbols {
		if !strings.Contains(bridgeText, "C."+symbol) {
			t.Errorf("%s has no low-level bridge", symbol)
		}
	}
}
