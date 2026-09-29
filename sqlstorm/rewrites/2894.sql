WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.ViewCount,
        p.CreationDate,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank,
        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes
    FROM Posts p
    WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
PostMetrics AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.Score,
        rp.ViewCount,
        rp.CreationDate,
        rp.Rank,
        rp.UpVotes,
        rp.DownVotes,
        CASE 
            WHEN rp.Score > 100 THEN 'High Score'
            WHEN rp.Score BETWEEN 50 AND 100 THEN 'Medium Score'
            ELSE 'Low Score' 
        END AS ScoreCategory
    FROM RankedPosts rp
    WHERE rp.Rank <= 10
),
CommentCounts AS (
    SELECT 
        p.Id AS PostId,
        COUNT(c.Id) AS TotalComments
    FROM Posts p
    LEFT JOIN Comments c ON p.Id = c.PostId
    GROUP BY p.Id
),
FinalMetrics AS (
    SELECT 
        pm.PostId,
        pm.Title,
        pm.Score,
        pm.ViewCount,
        pm.CreationDate,
        pm.Rank,
        pm.UpVotes,
        pm.DownVotes,
        pm.ScoreCategory,
        COALESCE(cc.TotalComments, 0) AS TotalComments,
        (pm.UpVotes - pm.DownVotes) AS NetVotes
    FROM PostMetrics pm
    LEFT JOIN CommentCounts cc ON pm.PostId = cc.PostId
)
SELECT 
    fm.PostId,
    fm.Title,
    fm.Score,
    fm.ViewCount,
    fm.CreationDate,
    fm.Rank,
    fm.UpVotes,
    fm.DownVotes,
    fm.ScoreCategory,
    fm.TotalComments,
    fm.NetVotes,
    CASE 
        WHEN fm.NetVotes >= 0 THEN 'Positive Engagement'
        ELSE 'Negative Engagement'
    END AS EngagementCategory
FROM FinalMetrics fm
WHERE fm.ScoreCategory = 'High Score' OR fm.TotalComments > 5
ORDER BY fm.ViewCount DESC, fm.Score DESC;