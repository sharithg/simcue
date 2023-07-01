package file

import (
	"fmt"
	"io"
	"os"
)

func WriteMessageToFile(data string, messageId string) error {
	f, err := os.Create(fmt.Sprintf("messages/%s.data", messageId))

	if err != nil {
		return err
	}

	defer f.Close()

	_, err2 := f.WriteString(fmt.Sprintf("%s\n", data))

	if err2 != nil {
		return err
	}

	return nil
}

func ReadMessageFromFile(messageId string) (string, error) {
	fileName := "messages/%s.data"
	// Open the file
	file, err := os.Open(fmt.Sprintf(fileName, messageId))
	if err != nil {
		fmt.Println("Error opening file:", err)
		return "", err
	}

	// Ensure the file gets closed after reading
	defer file.Close()

	// Read all contents of the file
	content, err := io.ReadAll(file)
	if err != nil {
		fmt.Println("Error reading file:", err)
		return "", err
	}

	// Close the file before deleting it
	file.Close()

	// Delete the file
	err = os.Remove(fmt.Sprintf(fileName, messageId))
	if err != nil {
		fmt.Println("Error deleting file:", err)
		return "", err
	}

	// Print the file contents as a string
	return string(content), nil
}
