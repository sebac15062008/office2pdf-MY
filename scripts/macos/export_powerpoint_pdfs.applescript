on run argv
    if (count argv) < 3 or ((count argv) mod 2) is 0 then error "usage: output-directory id input-path [id input-path ...]"

    set outputDirectory to item 1 of argv
    set failures to {}
    do shell script "mkdir -p " & quoted form of outputDirectory

    tell application "Microsoft PowerPoint"
        launch
        repeat with argumentIndex from 2 to (count argv) by 2
            set outputId to item argumentIndex of argv
            set inputPath to item (argumentIndex + 1) of argv
            set outputPath to outputDirectory & "/" & outputId & ".pdf"
            set inputFile to my posixFile(inputPath)
            set openedPresentation to missing value

            try
                with timeout of 120 seconds
                    open inputFile
                    set openedPresentation to active presentation
                    set outputFile to my createEmptyFile(outputPath)
                    save openedPresentation in outputFile as save as PDF
                    my waitForNonEmptyFile(outputPath)
                    close openedPresentation
                end timeout
            on error errorMessage number errorNumber
                if openedPresentation is not missing value then
                    try
                        close openedPresentation
                    end try
                end if
                set end of failures to outputId & ": " & errorMessage & " (" & errorNumber & ")"
            end try
        end repeat
        quit
    end tell

    if (count failures) > 0 then error my joinLines(failures)
end run

on posixFile(inputPath)
    return inputPath as POSIX file
end posixFile

on createEmptyFile(outputPath)
    do shell script ": > " & quoted form of outputPath
    return outputPath as POSIX file
end createEmptyFile

on waitForNonEmptyFile(outputPath)
    repeat 120 times
        if (do shell script "test -s " & quoted form of outputPath & " && echo yes || true") is "yes" then return
        delay 1
    end repeat
    error "PowerPoint did not create a non-empty PDF"
end waitForNonEmptyFile

on joinLines(values)
    set AppleScript's text item delimiters to linefeed
    set joined to values as text
    set AppleScript's text item delimiters to ""
    return joined
end joinLines
