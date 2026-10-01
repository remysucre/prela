WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.ViewCount,
        p.Score,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVotes,
        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVotes
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
PostDetails AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.CreationDate,
        rp.ViewCount,
        rp.Score,
        rp.UpVotes,
        rp.DownVotes,
        CASE 
            WHEN rp.Score > 0 THEN 'Positive' 
            WHEN rp.Score < 0 THEN 'Negative' 
            ELSE 'Neutral' 
        END AS ScoreType
    FROM 
        RankedPosts rp
    WHERE 
        rp.rn = 1
),
CommentCounts AS (
    SELECT 
        pc.PostId, 
        COUNT(*) AS TotalComments
    FROM 
        Comments pc
    GROUP BY 
        pc.PostId
),
CombinedData AS (
    SELECT 
        pd.PostId,
        pd.Title,
        pd.CreationDate,
        pd.ViewCount,
        pd.Score,
        pd.UpVotes,
        pd.DownVotes,
        pd.ScoreType,
        COALESCE(cc.TotalComments, 0) AS CommentCount
    FROM 
        PostDetails pd
    LEFT JOIN 
        CommentCounts cc ON pd.PostId = cc.PostId
)
SELECT 
    cd.*,
    CASE 
        WHEN cd.ViewCount IS NULL THEN 'Views Not Available'
        ELSE CONCAT(cd.ViewCount, ' views') 
    END AS ViewCountDetails,
    CASE 
        WHEN cd.CommentCount > 0 THEN 'Has Comments'
        ELSE 'No Comments'
    END AS CommentStatus,
    (SELECT STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) 
     FROM Tags t 
     JOIN Posts p ON p.Id = cd.PostId 
     WHERE p.Tags IS NOT NULL AND POSITION(t.TagName IN p.Tags) > 0) AS AssociatedTags
FROM 
    CombinedData cd
ORDER BY 
    cd.Score DESC NULLS LAST, 
    cd.ViewCount DESC NULLS FIRST
LIMIT 100 OFFSET 0;