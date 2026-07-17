on run argv
    if (count argv) < 3 or ((count argv) mod 2) is 0 then error "usage: output-directory id input-path [id input-path ...]"

    set outputDirectory to item 1 of argv
    set failures to {}
    do shell script "mkdir -p " & quoted form of outputDirectory

    tell application "Microsoft Excel"
        launch
        set display alerts to false
        repeat with argumentIndex from 2 to (count argv) by 2
            set outputId to item argumentIndex of argv
            set inputPath to item (argumentIndex + 1) of argv
            set openedWorkbook to missing value

            try
                my removePreviousSheetPdfs(outputDirectory, outputId)
                with timeout of 120 seconds
                    open workbook workbook file name inputPath update links do not update links read only true ignore read only recommended true
                    set openedWorkbook to active workbook
                    set visibleSheetCount to 0

                    repeat with sheetIndex from 1 to (count worksheets of openedWorkbook)
                        set currentSheet to worksheet sheetIndex of openedWorkbook
                        if visible of currentSheet is sheet visible then
                            set visibleSheetCount to visibleSheetCount + 1
                            set outputPath to outputDirectory & "/" & outputId & "-sheet-" & my paddedIndex(sheetIndex) & ".pdf"
                            set outputFile to my createEmptyFile(outputPath)
                            save as currentSheet filename outputFile file format PDF file format
                            my waitForNonEmptyFile(outputPath)
                        end if
                    end repeat

                    if visibleSheetCount is 0 then error "workbook has no visible worksheets"
                    close openedWorkbook saving no
                end timeout
            on error errorMessage number errorNumber
                if openedWorkbook is not missing value then
                    try
                        close openedWorkbook saving no
                    end try
                end if
                set end of failures to outputId & ": " & errorMessage & " (" & errorNumber & ")"
            end try
        end repeat
        quit
    end tell

    if (count failures) > 0 then error my joinLines(failures)
end run

on removePreviousSheetPdfs(outputDirectory, outputId)
    set filePattern to outputId & "-sheet-*.pdf"
    do shell script "find " & quoted form of outputDirectory & " -maxdepth 1 -type f -name " & quoted form of filePattern & " -delete"
end removePreviousSheetPdfs

on paddedIndex(sheetIndex)
    return do shell script "printf '%04d' " & sheetIndex
end paddedIndex

on createEmptyFile(outputPath)
    do shell script ": > " & quoted form of outputPath
    return outputPath as POSIX file
end createEmptyFile

on waitForNonEmptyFile(outputPath)
    repeat 120 times
        if (do shell script "test -s " & quoted form of outputPath & " && echo yes || true") is "yes" then return
        delay 1
    end repeat
    error "Excel did not create a non-empty PDF"
end waitForNonEmptyFile

on joinLines(values)
    set AppleScript's text item delimiters to linefeed
    set joined to values as text
    set AppleScript's text item delimiters to ""
    return joined
end joinLines
