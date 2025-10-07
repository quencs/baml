function AIContentPipeline(video_id: string, zoom_meeting_id: string, transcript: string?, title: string?) -> string {
    let emails = GetEmails();
    let posts = GetPosts();

    let processed = ProcessVideo(video_id, zoom_meeting_id);
    let video = GetVideo(video_id);


    # Generate Content
    let summary = if (transcript) {
        let s = SummarizeVideo(transcript, title);
        let category = LabelSummaryCategory(s);
        let saved_summary = SaveSummary(video_id, s);

        saved_summary
    } else {
        "no-transcript"
    };

    let content_types = ["email", "x", "linkedin"];

    for (let kind in content_types) {
        let structure = GetStructure(kind);
        let draft = GenerateContent(kind, video_id, summary);

        let saved_draft = SaveDraft(video_id, draft);

        saved_draft
    }

    let pr_status = if (summary) {
        let pr = CreatePR(video_id, summary);
        pr_status
    } else {
        "no-pr"
    };

    "done"
}
