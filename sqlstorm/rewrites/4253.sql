
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.ViewCount,
        p.CreationDate,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= CAST('2024-10-01' AS DATE) - INTERVAL '1 year'
),
RecentPosts AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.Score,
        rp.ViewCount,
        COALESCE(ph.CreationDate, CAST('1900-01-01' AS TIMESTAMP)) AS LastHistoryDate,
        CASE 
            WHEN ph.Comment IS NULL THEN 'No comments'
            ELSE ph.Comment
        END AS LastComment
    FROM 
        RankedPosts rp
    LEFT JOIN 
        PostHistory ph ON rp.PostId = ph.PostId AND ph.CreationDate = (
            SELECT MAX(ph2.CreationDate)
            FROM PostHistory ph2
            WHERE ph2.PostId = rp.PostId
        )
    WHERE 
        rp.Rank <= 5
),
UserEngagement AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        COUNT(c.Id) AS CommentCount
    FROM 
        Users u
    LEFT JOIN 
        Votes v ON v.UserId = u.Id
    LEFT JOIN 
        Comments c ON c.UserId = u.Id
    GROUP BY 
        u.Id, u.DisplayName
)
SELECT 
    rp.Title,
    rp.Score,
    rp.ViewCount,
    ue.DisplayName,
    ue.UpVotes,
    ue.DownVotes,
    rp.LastHistoryDate,
    rp.LastComment
FROM 
    RecentPosts rp
JOIN 
    UserEngagement ue ON ue.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId)
ORDER BY 
    rp.Score DESC, rp.ViewCount DESC, rp.PostId, rp.LastComment
LIMIT 10;
