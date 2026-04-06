# Requirements

## Feature F-1: File Input

### Rule R-1: Validate accepted file types are JPEG and PNG only
Example: Valid JPEG file is accepted
  Given a valid JPEG image file with extension `.jpg` in the drag payload
  When the drop event is processed
  Then the file is added to the processing queue
  And no error message is displayed

Example: Unsupported file format is rejected
  Given a GIF image file with extension `.gif` in the drag payload
  When the drop event is processed
  Then the file is not added to the processing queue
  And an inline error message reads "Unsupported file type"

### Rule R-2: Validate the per-file size limit is enforced at 50MB
Example: File within the size limit is accepted
  Given a JPEG file of 10MB is dropped onto the drop zone
  When the drop event is processed
  Then the file is added to the processing queue

Example: File exceeding the size limit is rejected
  Given a JPEG file of 60MB is dropped onto the drop zone
  When the drop event is processed
  Then the file is not added to the processing queue
  And an inline error message reads "Exceeds 50MB limit"

### Rule R-3: Validate batch input supports up to 50 files per session
Example: A batch of 20 files is accepted in full
  Given 20 JPEG files are present in the file picker selection
  When the selection is confirmed
  Then all 20 files appear in the processing queue

Example: A batch exceeding 50 files is capped with a warning
  Given 55 JPEG files are dropped onto the drop zone simultaneously
  When the drop event is processed
  Then the first 50 files are added to the processing queue
  And an inline warning states that only the first 50 files will be processed

---

## Feature F-2: JPEG Metadata Removal

### Rule R-4: Validate all EXIF APP1 segments are removed from JPEG output
Example: JPEG with GPS and device data produces a clean output file
  Given a JPEG file containing GPS latitude, GPS longitude, and an original capture timestamp
  When the processing core strips the file
  Then the output file contains no APP1 marker segments
  And exiftool reports zero EXIF tags on the output file

Example: JPEG with no EXIF data passes through as a valid JPEG
  Given a JPEG file containing no APP1 marker segments
  When the processing core strips the file
  Then the output file is a valid JPEG
  And no error is returned

### Rule R-5: Validate embedded thumbnails are removed from JPEG output
Example: JPEG with an embedded thumbnail produces a clean output
  Given a JPEG file containing an embedded EXIF thumbnail in the APP1 segment
  When the processing core strips the file
  Then the output file contains no embedded thumbnail
  And exiftool -b -ThumbnailImage on the output file returns no data

Example: JPEG without an embedded thumbnail is processed without error
  Given a JPEG file with EXIF data but no embedded thumbnail
  When the processing core strips the file
  Then the output file contains no EXIF data
  And no error is returned

### Rule R-6: Validate pixel data is preserved without re-encoding
Example: Pixel content of a stripped JPEG matches the input
  Given an 8-megapixel JPEG file with EXIF data
  When the processing core strips the file
  Then the output JPEG contains the same image content as the input
  And the output file size is smaller than the input by approximately the EXIF segment size

Example: A progressive JPEG structure is preserved after stripping
  Given a progressive JPEG file with EXIF data
  When the processing core strips the file
  Then the output is a valid progressive JPEG
  And a JPEG structure validator reports no errors on the output

---

## Feature F-3: PNG Metadata Removal

### Rule R-7: Validate metadata-bearing ancillary chunks are removed from PNG output
Example: PNG with tEXt and eXIf chunks produces a clean output file
  Given a PNG file containing tEXt, zTXt, iTXt, eXIf, and tIME chunks
  When the processing core strips the file
  Then the output PNG contains none of those chunk types
  And pngcheck -v on the output file reports no unknown ancillary chunks

Example: PNG with no metadata chunks passes through without modification
  Given a PNG file containing only IHDR, IDAT, and IEND chunks
  When the processing core strips the file
  Then the output file is a valid PNG
  And no error is returned

### Rule R-8: Validate ICC color profile and gamma chunks are preserved in PNG output
Example: PNG with iCCP and gAMA chunks retains those chunks after stripping
  Given a PNG file containing an iCCP color profile chunk and a gAMA gamma chunk alongside metadata chunks
  When the processing core strips the file
  Then the output PNG contains the iCCP chunk
  And the output PNG contains the gAMA chunk
  And no metadata chunk types are present in the output

Example: PNG with only iCCP and no metadata chunks is returned unchanged
  Given a PNG file containing only IHDR, iCCP, IDAT, and IEND chunks
  When the processing core strips the file
  Then the output file is byte-for-byte identical to the input

---

## Feature F-4: Metadata Diff Display

### Rule R-9: Validate metadata found in a file is displayed in categories before download
Example: JPEG with mixed metadata shows a categorised summary
  Given a JPEG file containing GPS coordinates, device make and model, and an original capture timestamp
  When processing completes
  Then the result row for that file shows a Location category entry with GPS values
  And a Device category entry with make and model
  And a Timestamps category entry with the capture timestamp value

Example: JPEG with no metadata shows a no-metadata message
  Given a JPEG file containing no EXIF tags
  When processing completes
  Then the result row displays "No metadata found"
  And no category entries are shown

### Rule R-10: Validate file size before and after processing is displayed per file
Example: File with metadata shows its size reduction
  Given a JPEG file of 4.2MB with substantial EXIF data is processed
  When processing completes
  Then the result row displays the original file size
  And the cleaned file size
  And the cleaned size is smaller than the original

Example: File with no metadata shows equal before and after sizes
  Given a JPEG file of 2.1MB with no EXIF data is processed
  When processing completes
  Then the result row displays the original and cleaned sizes as equal

---

## Feature F-5: Single File Download

### Rule R-11: Validate a single cleaned file is downloadable immediately after processing
Example: Single JPEG download is triggered successfully
  Given one JPEG file has been processed
  When the user activates the download button for that file
  Then the browser initiates a download
  And the downloaded file is named with the clean_ prefix followed by the original filename
  And the downloaded file contains no EXIF data

Example: A download button is not present before a file finishes processing
  Given a JPEG file has been added to the queue but processing has not completed
  When the result row is visible in the UI
  Then no download button or link is present for that file

### Rule R-12: Validate the output filename is prefixed with clean_ and retains the original extension
Example: JPEG output filename is correctly prefixed
  Given a file named photo_001.jpg is processed
  When the download link is generated
  Then the download filename is clean_photo_001.jpg

Example: PNG output filename is correctly prefixed
  Given a file named screenshot.png is processed
  When the download link is generated
  Then the download filename is clean_screenshot.png

Example: A file that fails processing does not produce a download link
  Given a file that the processing core cannot parse
  When the processing attempt completes with an error
  Then no download link is generated for that file
  And an error message is displayed in the result row

---

## Feature F-6: Batch Download

### Rule R-13: Validate all processed files can be downloaded as a single ZIP archive
Example: Batch ZIP download contains all cleaned files
  Given three JPEG files have been processed successfully
  When the user activates the Download All as ZIP button
  Then the browser initiates a download of a ZIP file
  And the ZIP contains three files each prefixed with clean_
  And each file in the ZIP contains no EXIF data

Example: The Download All as ZIP button is not available when no files have completed
  Given no files have been processed yet
  When the page is in the idle state
  Then the Download All as ZIP button is absent or disabled

---

## Feature F-7: Client-Side Processing Guarantee

### Rule R-14: Validate no network requests are initiated after the page has loaded
Example: Processing a file produces zero new outbound requests
  Given the page has fully loaded
  When a JPEG file is dropped onto the drop zone and processed
  Then zero new network requests appear in the browser network panel

Example: A Content Security Policy header blocks any attempted connection
  Given the page is served with Content-Security-Policy containing connect-src 'none'
  When any script in the page attempts to initiate a network request
  Then the browser blocks the request and logs a CSP violation

### Rule R-15: Validate no file data persists after the browser tab is closed
Example: Storage is empty after processing and closing the tab
  Given a JPEG file has been processed and downloaded
  When the browser tab is closed and reopened to the same page
  Then the browser's key-value storage contains no image data
  And the browser's client-side database contains no image data

Example: The application source contains no writes of image data to persistent storage
  Given the application source code is audited
  When all calls to persistent storage APIs are inspected
  Then none of those calls pass image file data as an argument

---

## Feature F-8: Performance

### Rule R-16: Validate single file processing completes within 100ms for files under 10MB
Example: An 8-megapixel JPEG is processed in under 100ms
  Given an 8-megapixel JPEG file of approximately 3MB
  When the processing core is called with that file
  Then the processing duration measured from call start to result is less than 100 milliseconds

Example: Processing time is surfaced in the result row
  Given a JPEG file has been processed
  When the result row is displayed
  Then a processing duration indicator is visible alongside the file entry

Example: A file that exceeds the processing time limit surfaces a warning
  Given a JPEG file whose processing exceeds 500 milliseconds
  When the processing result is received
  Then a slow-processing warning is shown alongside that file's result row

### Rule R-17: Validate the UI thread is not blocked during file processing
Example: The page remains interactive during a batch job
  Given 20 JPEG files of 5MB each are queued for processing
  When batch processing is running
  Then UI interactions including scrolling and button activation remain responsive
  And processing executes in a thread separate from the main UI thread

Example: A progress counter updates while batch processing is in progress
  Given 20 files are queued for processing
  When processing is in progress
  Then a counter showing the number of completed files out of the total is visible and incrementing

---

## Assumptions

| # | Assumption | Basis | Impact if wrong |
|---|-----------|-------|-----------------|
| A1 | Drag-and-drop FileList and DataTransfer APIs are present in all four target browsers | MDN compatibility tables as of 2026; Chrome 90+, Firefox 89+, Safari 15.2+, Edge 90+ all support these APIs in stable releases | A file-picker-only fallback would be required for any browser without drag-and-drop support |
| A2 | The Web Worker can import and initialise the WASM module without COOP or COEP headers on the demo hosting tier | Confirmed working in Chrome and Firefox for wasm-bindgen output without COOP; Safari 15.2+ supports it | If a target browser requires COOP for WASM in workers, the demo tier would require a different module loading strategy |
| A3 | The client-side ZIP library is fast enough to stay within the 3-second batch target for 20 × 5MB files | Based on published benchmark data for the planned library; not yet confirmed with in-browser profiling on target hardware | If packaging is too slow, store-mode ZIP or streaming generation would be required to meet the target |
| A4 | The metadata extraction function returning a structured array is sufficient for the diff display without additional formatting logic | Assumed based on the planned processing core API surface; not yet prototyped | If the core-to-browser data bridge introduces overhead, a separate lightweight parsing step may be needed |
