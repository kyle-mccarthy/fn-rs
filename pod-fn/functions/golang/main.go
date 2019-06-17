package main

import (
	"os"
	"io/ioutil"
	"fmt"
	"log"
)

func main() {
	input, err := ioutil.ReadAll(os.Stdin)

	if err != nil {
		log.Fatalf("Unable to read standard input: %s", err.Error())
	}

	incoming := string(input)

	fmt.Println(incoming)
}
