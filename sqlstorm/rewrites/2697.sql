WITH RankedPosts AS (
    SELECT 
        p.Id, 
        p.Title, 
        p.Score, 
        p.CreationDate, 
        u.DisplayName AS OwnerName,
        ROW_NUMBER() OVER (PARTITION BY CAST(p.CreationDate AS DATE) ORDER BY p.Score DESC, p.Id) AS RankScore
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND 
        p.PostTypeId = 1  
),
TopScorers AS (
    SELECT 
        rp.OwnerName, 
        COUNT(rp.Id) AS QuestionsCount
    FROM 
        RankedPosts rp
    WHERE 
        rp.RankScore <= 5
    GROUP BY 
        rp.OwnerName
),
PostComments AS (
    SELECT 
        p.Id AS PostId, 
        COUNT(c.Id) AS CommentsCount 
    FROM 
        Posts p 
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    GROUP BY 
        p.Id
),
PostDetails AS (
    SELECT 
        rp.Id, 
        rp.Title, 
        rp.Score, 
        pc.CommentsCount, 
        ts.QuestionsCount
    FROM 
        RankedPosts rp
    LEFT JOIN 
        PostComments pc ON rp.Id = pc.PostId
    LEFT JOIN 
        TopScorers ts ON rp.OwnerName = ts.OwnerName
)
SELECT 
    pd.Id, 
    pd.Title, 
    pd.Score, 
    pd.CommentsCount, 
    COALESCE(pd.QuestionsCount, 0) AS QuestionsCount, 
    CASE 
        WHEN pd.Score >= 100 THEN 'High Score'
        WHEN pd.Score >= 50 THEN 'Medium Score'
        ELSE 'Low Score'
    END AS ScoreCategory
FROM 
    PostDetails pd
WHERE 
    pd.CommentsCount > 10
ORDER BY 
    pd.Score DESC, pd.Title ASC;