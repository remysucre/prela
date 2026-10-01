
WITH PostStatistics AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        COUNT(DISTINCT c.Id) AS CommentCount,
        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId IN (2, 3)) AS VoteCount,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS PostRank
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
    GROUP BY 
        p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId
),
TopPosts AS (
    SELECT 
        ps.PostId,
        ps.Title,
        ps.CreationDate,
        ps.Score,
        ps.ViewCount,
        ps.CommentCount,
        ps.VoteCount,
        ps.UpVotes,
        ps.DownVotes,
        ps.PostRank,
        DENSE_RANK() OVER (ORDER BY ps.Score DESC) AS ScoreRank
    FROM 
        PostStatistics ps
),
FilteredPosts AS (
    SELECT 
        tp.*,
        CASE 
            WHEN tp.CommentCount = 0 THEN 'No Comments'
            WHEN tp.UpVotes > tp.DownVotes THEN 'Positive Feedback'
            ELSE 'Needs Improvement'
        END AS FeedbackStatus
    FROM 
        TopPosts tp
    WHERE 
        tp.Score > 0
        AND tp.PostRank <= 5
    ORDER BY 
        tp.ScoreRank
)
SELECT 
    fp.PostId,
    fp.Title,
    fp.CreationDate,
    fp.Score,
    fp.ViewCount,
    fp.CommentCount,
    fp.VoteCount,
    fp.UpVotes,
    fp.DownVotes,
    fp.FeedbackStatus,
    CASE 
        WHEN fp.FeedbackStatus = 'No Comments' THEN 'Consider adding engaging content!'
        ELSE 'Great contribution!'
    END AS Advice
FROM 
    FilteredPosts fp
WHERE 
    fp.FeedbackStatus IS NOT NULL
ORDER BY 
    fp.Score DESC, 
    fp.CreationDate DESC;
