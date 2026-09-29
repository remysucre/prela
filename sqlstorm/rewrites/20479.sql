
WITH PostDetails AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.CreationDate,
        COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId,
        p.ViewCount,
        COUNT(c.Id) AS CommentCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
        STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON c.PostId = p.Id
    LEFT JOIN 
        Votes v ON v.PostId = p.Id
    LEFT JOIN 
        LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS TagName) AS tag ON true
    LEFT JOIN 
        Tags t ON t.TagName = tag.TagName
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' 
        AND p.ViewCount > 0
    GROUP BY 
        p.Id, p.Title, p.Score, p.CreationDate, p.AcceptedAnswerId, p.ViewCount
),
RankedPosts AS (
    SELECT 
        pd.*,
        ROW_NUMBER() OVER (PARTITION BY pd.AcceptedAnswerId ORDER BY pd.Score DESC, pd.PostId) AS Rank
    FROM 
        PostDetails pd
),
PostsWithBadges AS (
    SELECT 
        rp.*,
        b.Name AS BadgeName,
        COUNT(b.Id) OVER (PARTITION BY rp.PostId) AS BadgeCount
    FROM 
        RankedPosts rp
    LEFT JOIN 
        Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId)
),
FinalOutput AS (
    SELECT 
        pwb.PostId,
        pwb.Title,
        pwb.Score,
        pwb.CommentCount,
        pwb.UpVoteCount,
        pwb.DownVoteCount,
        pwb.BadgeName,
        CASE 
            WHEN pwb.BadgeCount IS NULL THEN 'No Badge'
            ELSE 'Has Badge'
        END AS BadgeStatus
    FROM 
        PostsWithBadges pwb
    WHERE 
        pwb.Rank = 1
)

SELECT 
    *,
    (CASE 
        WHEN BadgeStatus = 'Has Badge' THEN 'Congratulations on your achievement!'
        ELSE 'Keep striving for that badge!'
    END) AS EncouragementMessage
FROM 
    FinalOutput
ORDER BY 
    CommentCount DESC NULLS LAST, 
    Score DESC;
