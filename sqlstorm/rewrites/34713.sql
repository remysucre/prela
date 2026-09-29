
WITH RecentPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.OwnerUserId,
        p.ViewCount,
        p.Score,
        p.AnswerCount,
        p.CommentCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS RN
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'
),
PostVoteSummary AS (
    SELECT 
        p.Id AS PostId,
        SUM(CASE WHEN v.VoteTypeId IN (2, 6) THEN 1 ELSE 0 END) AS TotalUpvotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
    FROM 
        Posts p
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    GROUP BY 
        p.Id
),
ClosedPosts AS (
    SELECT 
        ph.PostId,
        ph.CreationDate AS CloseDate,
        ph.UserDisplayName AS ClosedBy,
        ph.Comment
    FROM 
        PostHistory ph
    WHERE 
        ph.PostHistoryTypeId = 10
),
Metrics AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.CreationDate,
        rp.ViewCount,
        COALESCE(pvs.TotalUpvotes, 0) AS Upvotes,
        COALESCE(pvs.TotalDownvotes, 0) AS Downvotes,
        COALESCE(cp.CloseDate, NULL) AS ClosedDate,
        COALESCE(cp.ClosedBy, 'Open') AS ClosedBy,
        COALESCE(cp.Comment, 'N/A') AS CloseReason,
        (rp.ViewCount * 1.0 / NULLIF(rp.AnswerCount, 0)) AS ViewsPerAnswer,
        rp.Score
    FROM 
        RecentPosts rp
    LEFT JOIN 
        PostVoteSummary pvs ON rp.PostId = pvs.PostId
    LEFT JOIN 
        ClosedPosts cp ON rp.PostId = cp.PostId
    WHERE 
        rp.RN = 1
)
SELECT 
    m.PostId,
    m.Title,
    m.CreationDate,
    m.ViewCount,
    m.Upvotes,
    m.Downvotes,
    m.ClosedDate,
    m.ClosedBy,
    m.CloseReason,
    m.ViewsPerAnswer,
    CASE 
        WHEN m.ClosedDate IS NOT NULL THEN 'Closed'
        ELSE 'Open'
    END AS PostStatus
FROM 
    Metrics m
ORDER BY 
    m.Score DESC, 
    m.ViewCount DESC, m.PostId, m.ClosedDate 
LIMIT 100;
