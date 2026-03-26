package file_util

import (
	"bufio"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"microscope/config"
	"microscope/util/side_channel"
)

func NonEmpty(filename string) bool {
	info, err := os.Stat(filename)
	if err != nil {
		return false
	}
	if !info.Mode().IsRegular() {
		return false
	}
	return info.Size() > 0
}

func writeFile(filename string, iter func(f func(i int, val []rune) bool)) error {
	file, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer file.Close()
	writer := bufio.NewWriter(file)
	defer writer.Flush()

	for _, line := range iter {
		_, err = writer.WriteString(string(line) + "\n")
		if err != nil {
			return err
		}
	}
	err = writer.Flush()
	if err != nil {
		return err
	}
	err = file.Close()
	if err != nil {
		return err
	}
	return nil
}

func copyFile(src string, dst string) error {
	srcFile, err := os.Open(src)
	if err != nil {
		return err
	}
	defer srcFile.Close()

	dstFile, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer dstFile.Close()

	if _, err := io.Copy(dstFile, srcFile); err != nil {
		return err
	}
	err = srcFile.Close()
	if err != nil {
		return err
	}
	err = dstFile.Close()
	if err != nil {
		return err
	}

	return nil
}
func moveFile(dst string, src string) error {
	// save mode
	var dstMode fs.FileMode = 0600
	if dstInfo, err := os.Stat(dst); err == nil {
		dstMode = dstInfo.Mode()
	}

	// save file
	err := os.Rename(src, dst)
	if err != nil {
		// try copy file content
		err = copyFile(src, dst)
		if err != nil {
			return err
		}

		return os.Remove(src)
	}

	// restore mode
	err = os.Chmod(dst, dstMode)
	if err != nil {
		return err
	}
	return nil
}

func SafeWriteFile(filename string, iter func(f func(i int, val []rune) bool)) error {
	absPath, _ := filepath.Abs(filename)
	tmpFilename := filepath.Join(config.Load().TMP_DIR, absPath)

	// make tmp dir
	err := os.MkdirAll(filepath.Dir(tmpFilename), 0o700)
	if err != nil {
		side_channel.WriteLn(err)
		return err
	}

	// write into tmp file
	err = writeFile(tmpFilename, iter)
	if err != nil {
		side_channel.WriteLn(err)
		return err
	}

	// move tmp file into output file
	err = moveFile(filename, tmpFilename)
	if err != nil {
		side_channel.WriteLn(err)
		return err
	}
	return nil
}
