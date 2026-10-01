
WITH UserScoreStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        u.Reputation,
        u.Views,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
        COUNT(v.Id) AS TotalVotes
    FROM 
        Users u
    LEFT JOIN 
        Votes v ON u.Id = v.UserId
    GROUP BY 
        u.Id, u.DisplayName, u.Reputation, u.Views
),
PostActivity AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        u.DisplayName AS Author,
        p.CreationDate,
        v.Id AS VoteId,
        COALESCE(c.CommentCount, 0) AS Comments,
        COALESCE(a.AnswerCount, 0) AS Answers,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS TotalUpvotes,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS TotalDownvotes
    FROM 
        Posts p
    LEFT JOIN 
        Users u ON p.OwnerUserId = u.Id
    LEFT JOIN 
        (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
    LEFT JOIN 
        (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    WHERE 
        p.ViewCount IS NOT NULL
),
FinalMetrics AS (
    SELECT 
        psa.UserId,
        psa.DisplayName,
        p.PostId,
        p.VoteId,
        ps.Id AS PsId,
        p.Title,
        p.Author,
        p.CreationDate,
        p.Comments,
        p.Answers,
        p.TotalUpvotes,
        p.TotalDownvotes,
        COALESCE(ps.Reputation, 0) AS ReputationScore,
        CASE 
            WHEN ps.Reputation > 1000 THEN 'Gold'
            WHEN ps.Reputation BETWEEN 500 AND 1000 THEN 'Silver'
            ELSE 'Bronze'
        END AS BadgeType,
        LEAD(ps.Reputation) OVER (ORDER BY p.CreationDate DESC, p.PostId, p.VoteId, psa.UserId, ps.Id) AS NextUserReputation
    FROM 
        PostActivity p
    LEFT JOIN 
        UserScoreStats psa ON psa.DisplayName = p.Author
    LEFT JOIN 
        Users ps ON p.Author = ps.DisplayName
    WHERE 
        p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 YEAR'
)
SELECT 
    fm.UserId,
    fm.DisplayName,
    fm.Title,
    fm.Author,
    fm.CreationDate,
    fm.Comments,
    fm.Answers,
    fm.TotalUpvotes,
    fm.TotalDownvotes,
    fm.ReputationScore,
    fm.BadgeType,
    CASE 
        WHEN fm.ReputationScore IS NULL THEN 'No reputation'
        WHEN fm.ReputationScore < 100 THEN 'Newbie'
        ELSE NULL
    END AS ReputationMessage,
    CASE 
        WHEN fm.NextUserReputation IS NOT NULL THEN 
            CASE 
                WHEN fm.NextUserReputation > fm.ReputationScore THEN 'Next user has higher reputation'
                ELSE 'Next user has lower or equal reputation'
            END 
        ELSE 'No subsequent user record'
    END AS UserComparison
FROM 
    FinalMetrics fm
WHERE 
    fm.TotalUpvotes > fm.TotalDownvotes
    OR (fm.TotalUpvotes IS NULL AND fm.TotalDownvotes IS NULL)
ORDER BY 
    fm.ReputationScore DESC, fm.CreationDate DESC, fm.PostId, fm.VoteId, fm.UserId, fm.PsId
FETCH FIRST 10 ROWS ONLY;
