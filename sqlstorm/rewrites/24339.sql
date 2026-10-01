
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id, c.Id) AS ScoreRank,
        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
        LEAD(p.Score) OVER (ORDER BY p.CreationDate, p.Id, c.Id) AS NextPostScore,
        p.OwnerUserId
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    WHERE 
        p.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND
        p.Score IS NOT NULL
),
PostDetails AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.CreationDate,
        rp.Score,
        rp.AcceptedAnswerId,
        rp.ScoreRank,
        rp.CommentCount,
        CASE 
            WHEN rp.Score < COALESCE(rp.NextPostScore, 0) THEN 'Improving'
            WHEN rp.Score = 0 THEN 'Neutral'
            ELSE 'Declining' 
        END AS ScoreTrend
    FROM 
        RankedPosts rp
    WHERE 
        rp.ScoreRank <= 5 
        AND NOT EXISTS (
            SELECT 1 
            FROM Posts p 
            WHERE p.OwnerUserId = rp.OwnerUserId AND p.Score > rp.Score
        )
),
BadgeCounts AS (
    SELECT 
        b.UserId, 
        COUNT(b.Id) AS TotalBadges
    FROM 
        Badges b
    WHERE 
        b.Class = 1  
    GROUP BY 
        b.UserId
)
SELECT 
    ud.Id AS UserId,
    ud.DisplayName,
    pd.Title,
    pd.Score,
    pd.ScoreTrend,
    pd.CommentCount,
    COALESCE(bc.TotalBadges, 0) AS GoldBadges
FROM 
    Users ud
JOIN 
    PostDetails pd ON ud.Id = (SELECT OwnerUserId FROM Posts WHERE Id = pd.PostId)
LEFT JOIN 
    BadgeCounts bc ON ud.Id = bc.UserId
WHERE 
    pd.ScoreTrend = 'Declining' 
    AND (ud.Reputation > 1000 OR ud.Views < 50)
ORDER BY 
    pd.CommentCount DESC, 
    bc.TotalBadges DESC, pd.PostId
OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
